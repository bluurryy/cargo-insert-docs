#[cfg(test)]
mod tests;

use core::{fmt::Write, ops::Range};
use std::collections::{HashMap, HashSet};

use crate::{
    markdown::{byte_range, parse, parse_options},
    markdown_rs::event::{Event, Kind, Name},
    string_replacer::StringReplacer,
};

#[derive(Default)]
pub struct RewriteMarkdownOptions {
    pub shrink_headings: i8,
    pub links: Vec<(String, Option<String>)>,
}

pub fn rewrite_markdown(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    let markdown = add_definitions(markdown, options);
    rewrite(&markdown, options)
}

/// If we were not able to resolve an item, then it will get this placeholder destination.
/// A definition with a placeholder destination will be removed from the final markdown.
/// We need to temporarily create a definition with a placeholder destination, so
/// these references actually parse as references. Those references will then get
/// replaced with their label only.
const PLACEHOLDER_DESTINATION: &str = "__PLACEHOLDER_DESTINATION__";

fn add_definitions(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    let mut markdown = markdown.to_string();

    if !options.links.is_empty() {
        markdown.push_str("\n\n");
    }

    for (identifier, destination) in &options.links {
        let destination = destination.as_deref().unwrap_or(PLACEHOLDER_DESTINATION);
        markdown.write_fmt(format_args!("[{identifier}]: {destination}\n")).unwrap();
    }

    markdown
}

fn rewrite(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    let links: &HashMap<&str, Option<&str>> =
        &options.links.iter().map(|(k, v)| (k.as_str(), v.as_deref())).collect();

    let (events, _state) = parse(markdown, &parse_options());
    let events = events.as_slice();

    if events.is_empty() {
        return markdown.into();
    }

    let mut out = StringReplacer::new(markdown);
    let unused_definitions = unused_definitions(markdown, events, options);

    for index in (0..events.len()).rev() {
        let event = &events[index];

        if event.kind != Kind::Exit {
            continue;
        }

        match event.name {
            Name::HeadingAtx => {
                let Some(hashes) = child(events, index, Name::HeadingAtxSequence) else {
                    continue;
                };

                let hashes = byte_range(events, hashes);
                let level = hashes.len() as i8;
                let new_level = level.saturating_add(options.shrink_headings).clamp(1, 6);
                let new_hashes = &"######"[..new_level as usize];
                out.replace(hashes, new_hashes);
            }
            Name::CodeFenced => {
                if let Some(fence_info) = descendant(events, index, Name::CodeFencedFenceInfo) {
                    let fence_info_range = byte_range(events, fence_info);

                    if !code_block_fence_is_rust(&markdown[fence_info_range.clone()]) {
                        continue;
                    }

                    for child in children(events, index) {
                        if events[child].name == Name::CodeFlowChunk {
                            clean_code_chunk(&mut out, markdown, byte_range(events, child));
                        }
                    }

                    out.replace(fence_info_range, "rust");
                } else if let Some(fence) =
                    descendants_with_name(events, index, Name::CodeFencedFenceSequence).nth(1)
                {
                    let insert_point = byte_range(events, fence).end;

                    for child in children(events, index) {
                        if events[child].name == Name::CodeFlowChunk {
                            clean_code_chunk(&mut out, markdown, byte_range(events, child));
                        }
                    }

                    out.insert(insert_point, "rust");
                }
            }
            Name::CodeIndented => {
                let range = byte_range(events, index);
                out.insert(range.end, "\n```");

                for child in children(events, index) {
                    match events[child].name {
                        Name::SpaceOrTab => out.remove(byte_range(events, child)),
                        Name::CodeFlowChunk => {
                            clean_code_chunk(&mut out, markdown, byte_range(events, child));
                        }
                        _ => (),
                    }
                }

                out.insert(range.start, "```rust\n");
            }
            Name::Link => {
                let Some(label) = child(events, index, Name::Label) else {
                    continue;
                };

                if let Some(resource) = child(events, index, Name::Resource) {
                    let Some(dest) = child(events, resource, Name::ResourceDestination) else {
                        continue;
                    };

                    let Some(dest_string) =
                        descendant(events, dest, Name::ResourceDestinationString)
                    else {
                        continue;
                    };

                    let Some(&resolved) = links.get(&markdown[byte_range(events, dest_string)])
                    else {
                        continue;
                    };

                    let Some(new_url) = resolved else {
                        let Some(label_text) = child(events, label, Name::LabelText) else {
                            continue;
                        };

                        let range = byte_range(events, index);
                        out.replace(range, &markdown[byte_range(events, label_text)]);
                        continue;
                    };

                    let range = byte_range(events, dest);
                    out.replace(range, new_url);
                    // TODO: correctly escape / add angled brackets
                    continue;
                }

                if let Some(reference) = child(events, index, Name::Reference) {
                    let identifier = match child(events, reference, Name::ReferenceString) {
                        Some(string) => &markdown[byte_range(events, string)],
                        None => match child(events, label, Name::LabelText) {
                            Some(label_text) => &markdown[byte_range(events, label_text)],
                            None => continue,
                        },
                    };

                    let Some(&resolved) = links.get(identifier) else {
                        continue;
                    };

                    let Some(new_url) = resolved else {
                        let Some(label_text) = child(events, label, Name::LabelText) else {
                            continue;
                        };

                        let range = byte_range(events, index);
                        out.replace(range, &markdown[byte_range(events, label_text)]);
                        continue;
                    };

                    // refers to a definition
                    _ = new_url;
                    continue;
                }

                // shortcut
                let Some(label_text) = child(events, label, Name::LabelText) else {
                    continue;
                };

                let Some(&resolved) = links.get(&markdown[byte_range(events, label_text)]) else {
                    continue;
                };

                let Some(new_url) = resolved else {
                    let Some(label_text) = child(events, label, Name::LabelText) else {
                        continue;
                    };

                    let range = byte_range(events, index);
                    out.replace(range, &markdown[byte_range(events, label_text)]);
                    continue;
                };

                // refers to a definition
                _ = new_url;
            }
            Name::Definition => {
                let Some(dest) = child(events, index, Name::DefinitionDestination) else {
                    continue;
                };

                let Some(dest_string) = descendant(events, dest, Name::DefinitionDestinationString)
                else {
                    continue;
                };

                let Some(label) = descendant(events, index, Name::DefinitionLabelString) else {
                    continue;
                };

                let dest_string_str = &markdown[byte_range(events, dest_string)];
                let label_str = &markdown[byte_range(events, label)];

                if dest_string_str == PLACEHOLDER_DESTINATION
                    || unused_definitions.contains(label_str)
                {
                    let mut range = byte_range(events, index);
                    range.end = end_of_line(markdown, range.end);
                    out.remove(range);
                    continue;
                }

                let Some(&resolved) = links.get(dest_string_str) else {
                    continue;
                };

                let Some(new_url) = resolved else {
                    // let range = byte_range(events, index);
                    // out.remove(range);
                    // TODO: remove newline
                    continue;
                };

                let range = byte_range(events, dest);
                out.replace(range, new_url);
                // TODO: correctly escape / add angled brackets
            }
            _ => (),
        }
    }

    out.finish()
}

fn unused_definitions<'a>(
    markdown: &'a str,
    events: &[Event],
    options: &'a RewriteMarkdownOptions,
) -> HashSet<&'a str> {
    let mut used_definitions: HashSet<&str> = HashSet::new();

    for index in (0..events.len()).rev() {
        let event = &events[index];

        if event.kind != Kind::Exit {
            continue;
        }

        if event.name != Name::Link {
            continue;
        }

        if descendant(events, index, Name::Resource).is_some() {
            continue;
        }

        let identifier = match descendant(events, index, Name::ReferenceString) {
            Some(some) => some,
            None => match descendant(events, index, Name::LabelText) {
                Some(some) => some,
                None => continue,
            },
        };

        let range = byte_range(events, identifier);
        let str = &markdown[range];

        used_definitions.insert(str);
    }

    let all_definitions: HashSet<&str> = options.links.iter().map(|(k, _)| k.as_str()).collect();
    all_definitions.difference(&used_definitions).copied().collect()
}

fn end_of_line(markdown: &str, index: usize) -> usize {
    match markdown[index..].bytes().position(|b| b == b'\n') {
        Some(i) => index + i + 1,
        None => markdown.len(),
    }
}

fn clean_code_chunk(out: &mut StringReplacer, markdown: &str, range: Range<usize>) {
    let line = &markdown[range.clone()];
    let line_trim_start = line.trim_start();

    if let Some(rest) = line_trim_start.strip_prefix('#') {
        match rest.bytes().next() {
            Some(b' ') | None => {
                out.remove(range.start..end_of_line(markdown, range.end));
            }
            Some(b'#') => {
                // double hash `##`, remove one of the hashes
                let mid = range.start + substr_range(line, line_trim_start).start;
                out.remove(mid..mid + 1);
            }
            Some(_) => (),
        }
    }
}

fn substr_range(str: &str, substr: &str) -> Range<usize> {
    let start = substr.as_ptr() as usize - str.as_ptr() as usize;
    let end = start + substr.len();
    start..end
}

fn descendant(events: &[Event], index: usize, name: Name) -> Option<usize> {
    descendants_with_name(events, index, name).next()
}

fn child(events: &[Event], index: usize, name: Name) -> Option<usize> {
    children_with_name(events, index, name).next()
}

fn descendants_with_name(
    events: &[Event],
    index: usize,
    name: Name,
) -> impl Iterator<Item = usize> {
    descendants(events, index).filter(move |&i| events[i].name == name)
}

fn children_with_name(events: &[Event], index: usize, name: Name) -> impl Iterator<Item = usize> {
    children(events, index).filter(move |&i| events[i].name == name)
}

fn descendants(events: &[Event], index: usize) -> impl Iterator<Item = usize> {
    let mut depth = 0;

    (0..index)
        .rev()
        .take_while(move |&i| {
            let kind = events[i].kind.clone();

            if depth == 0 && kind == Kind::Enter {
                return false;
            }

            match kind {
                Kind::Enter => depth -= 1,
                Kind::Exit => depth += 1,
            }

            true
        })
        .filter(|&i| events[i].kind == Kind::Exit)
}

fn children(events: &[Event], index: usize) -> impl Iterator<Item = usize> {
    let mut depth = 0;

    (0..index)
        .rev()
        .map_while(move |i| {
            let kind = events[i].kind.clone();

            if depth == 0 && kind == Kind::Enter {
                return None;
            }

            match kind {
                Kind::Enter => depth -= 1,
                Kind::Exit => depth += 1,
            }

            Some((i, depth))
        })
        .filter_map(|(i, depth)| (depth == 1 && events[i].kind == Kind::Exit).then_some(i))
}

fn code_block_fence_is_rust(info: &str) -> bool {
    const STARTS: &[&str] = &[
        "rust",
        "ignore",
        "should_panic",
        "no_run",
        "compile_fail",
        "edition",
        "standalone_crate",
    ];

    if info.is_empty() {
        return true;
    }

    for start in STARTS {
        if info.starts_with(start) {
            return true;
        }
    }

    false
}
