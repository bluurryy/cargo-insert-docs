#[cfg(test)]
mod tests;

use core::{fmt::Write, ops::Range};
use std::collections::HashMap;

use crate::{
    markdown_rs::{
        self,
        event::{Event, Kind, Name},
        unist::Position,
    },
    string_replacer::StringReplacer,
};

#[derive(Default)]
pub struct RewriteMarkdownOptions {
    pub shrink_headings: i8,
    pub links: HashMap<String, Option<String>>,
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
        markdown.push('\n');
    }

    for (identifier, destination) in &options.links {
        let destination = destination.as_deref().unwrap_or(PLACEHOLDER_DESTINATION);
        markdown.write_fmt(format_args!("[{identifier}]: {destination}\n")).unwrap();
    }

    markdown
}

fn rewrite(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    let parse_options = markdown_rs::ParseOptions::gfm();

    let (events, _state) = markdown_rs::parser::parse(markdown, &parse_options)
        .expect("should only fail for mdx which we don't enable");

    let events = events.as_slice();

    if events.is_empty() {
        return markdown.into();
    }

    let mut out = StringReplacer::new(markdown);
    let mut index = events.len();

    const INTERESTING: &[Name] =
        &[Name::HeadingAtx, Name::CodeFenced, Name::CodeIndented, Name::Definition, Name::Link];

    while let Some(new_index) = find_any_of(events, index, INTERESTING) {
        index = new_index;
        process_one(&mut out, options, markdown, events, index);
    }

    out.finish()
}

fn process_one<'a>(
    out: &mut StringReplacer<'a>,
    options: &RewriteMarkdownOptions,
    markdown: &'a str,
    events: &[Event],
    index: usize,
) {
    match &events[index].name {
        Name::HeadingAtx => {
            let hashes = find(events, index, Name::HeadingAtxSequence);
            let hashes = byte_range(events, hashes);
            let level = hashes.len() as i8;
            let new_level = level.saturating_add(options.shrink_headings).clamp(1, 6);
            let new_hashes = &"######"[..new_level as usize];
            out.replace(hashes, new_hashes);
        }
        Name::CodeFenced => {
            if let Some(fence_info) = find_descendant(events, index, Name::CodeFencedFenceInfo) {
                let fence_info_range = byte_range(events, fence_info);

                if !code_block_fence_is_rust(&markdown[fence_info_range.clone()]) {
                    return;
                }

                for child in children(events, index) {
                    dbg!(&events[child]);

                    if events[child].name == Name::CodeFlowChunk {
                        clean_code_chunk(out, markdown, byte_range(events, child));
                    }
                }

                out.replace(fence_info_range, "rust");
            } else if let Some(fence) =
                descendants_with_name(events, index, Name::CodeFencedFenceSequence).nth(1)
            {
                let insert_point = byte_range(events, fence).end;

                for child in children(events, index) {
                    dbg!(&events[child]);

                    if events[child].name == Name::CodeFlowChunk {
                        clean_code_chunk(out, markdown, byte_range(events, child));
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
                        clean_code_chunk(out, markdown, byte_range(events, child));
                    }
                    _ => (),
                }
            }

            out.insert(range.start, "```rust\n");
        }
        Name::Link => {
            let Some(label) = find_child(events, index, Name::Label) else {
                return;
            };

            if let Some(resource) = find_child(events, index, Name::Resource) {
                let Some(dest) = find_child(events, resource, Name::ResourceDestination) else {
                    return;
                };

                let Some(dest_string) = find_child(events, dest, Name::ResourceDestinationString)
                else {
                    return;
                };

                let Some(resolved) = options.links.get(&markdown[byte_range(events, dest_string)])
                else {
                    return;
                };

                let Some(new_url) = resolved else {
                    let Some(label_text) = find_child(events, label, Name::LabelText) else {
                        return;
                    };

                    let range = byte_range(events, index);
                    out.replace(range, &markdown[byte_range(events, label_text)]);
                    return;
                };

                let range = byte_range(events, dest);
                out.replace(range, new_url.clone());
                // TODO: correctly escape / add angled brackets
                return;
            }

            if let Some(reference) = find_child(events, index, Name::Reference) {
                let Some(reference_string) = find_child(events, reference, Name::ReferenceString)
                else {
                    return;
                };

                let Some(resolved) =
                    options.links.get(&markdown[byte_range(events, reference_string)])
                else {
                    return;
                };

                let Some(new_url) = resolved else {
                    let Some(label_text) = find_child(events, label, Name::LabelText) else {
                        return;
                    };

                    let range = byte_range(events, index);
                    out.replace(range, &markdown[byte_range(events, label_text)]);
                    return;
                };

                let range = byte_range(events, reference);
                out.replace(range, format!("({new_url})"));
                // TODO: correctly escape / add angled brackets
                return;
            }

            // shortcut
            let Some(label_text) = find_child(events, label, Name::LabelText) else {
                return;
            };

            let Some(resolved) = options.links.get(&markdown[byte_range(events, label_text)])
            else {
                return;
            };

            let Some(new_url) = resolved else {
                let Some(label_text) = find_child(events, label, Name::LabelText) else {
                    return;
                };

                let range = byte_range(events, index);
                out.replace(range, &markdown[byte_range(events, label_text)]);
                return;
            };

            let label_text_str = &markdown[byte_range(events, label_text)];
            let range = byte_range(events, index);
            out.replace(range, format!("[{label_text_str}]({new_url})"));
            // TODO: correctly escape / add angled brackets
        }
        Name::Definition => {
            let dest = find(events, index, Name::DefinitionDestination);
            let dest_string = find(events, dest, Name::DefinitionDestinationString);
            let dest_string_range = byte_range(events, dest_string);
            let dest_string_str = &markdown[dest_string_range];

            let Some(resolved) = options.links.get(dest_string_str) else {
                return;
            };

            let Some(new_url) = resolved else {
                let range = byte_range(events, index);
                out.remove(range);
                // TODO: remove newline
                return;
            };

            let range = byte_range(events, dest);
            out.replace(range, new_url.clone());
            // TODO: correctly escape / add angled brackets
        }
        _ => unreachable!(),
    }
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
                let mid = substr_range(line, line_trim_start).start;
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

fn find_descendant(events: &[Event], index: usize, name: Name) -> Option<usize> {
    descendants_with_name(events, index, name).next()
}

fn find_child(events: &[Event], index: usize, name: Name) -> Option<usize> {
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

fn find_any_of(events: &[Event], index: usize, names: &[Name]) -> Option<usize> {
    (0..index).rev().find(|&index| {
        let event = &events[index];
        event.kind == Kind::Exit && names.contains(&event.name)
    })
}

fn find(events: &[Event], index: usize, name: Name) -> usize {
    let new_index = (0..index).rev().find(|&index| {
        let event = &events[index];
        event.kind == Kind::Exit && event.name == name
    });

    if let Some(new_index) = new_index {
        new_index
    } else {
        panic!("expected a markdown event of type {name:?}");
    }
}

fn position(events: &[Event], exit_index: usize) -> Position {
    let event = &events[exit_index];
    let end = event.point.to_unist();
    let name = event.name.clone();
    let enter_index = (0..exit_index)
        .rev()
        .find(|&index| {
            let event = &events[index];
            event.kind == Kind::Enter && event.name == name
        })
        .expect("unpaired enter/exit event");
    let start = events[enter_index].point.to_unist();
    Position { start, end }
}

fn byte_range(events: &[Event], index: usize) -> Range<usize> {
    let pos = position(events, index);
    pos.start.offset..pos.end.offset
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
