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
    let links: HashMap<&str, Option<&str>> =
        options.links.iter().map(|(k, v)| (k.as_str(), v.as_deref())).collect();

    let (events, _state) = parse(markdown, &parse_options());
    let events = events.as_slice();

    if events.is_empty() {
        return markdown.into();
    }

    let mut out = StringReplacer::new(markdown);
    let unused_definitions = unused_definitions(markdown, events, options);
    let tree = Tree { markdown, events };

    for node in tree.depth_first() {
        match node.name() {
            Name::HeadingAtx => {
                let Some(hashes) = node.child(Name::HeadingAtxSequence) else {
                    continue;
                };

                let hashes = hashes.byte_range();
                let level = hashes.len() as i8;
                let new_level = level.saturating_add(options.shrink_headings).clamp(1, 6);
                let new_hashes = &"######"[..new_level as usize];
                out.replace(hashes, new_hashes);
            }
            Name::CodeFenced => {
                if let Some(fence_info) = node.descendant(Name::CodeFencedFenceInfo) {
                    if !code_block_fence_is_rust(fence_info.str()) {
                        continue;
                    }

                    for child in node.children() {
                        if child.name() == Name::CodeFlowChunk {
                            clean_code_chunk(&mut out, markdown, child.byte_range());
                        }
                    }

                    out.replace(fence_info.byte_range(), "rust");
                } else if let Some(fence) =
                    node.descendants_with_name(Name::CodeFencedFenceSequence).nth(1)
                {
                    for child in node.children() {
                        if child.name() == Name::CodeFlowChunk {
                            clean_code_chunk(&mut out, markdown, child.byte_range());
                        }
                    }

                    out.insert(fence.byte_range().end, "rust");
                }
            }
            Name::CodeIndented => {
                let range = node.byte_range();
                out.insert(range.end, "\n```");

                let mut last_event_was_code_flow_chunk = false;

                for child in node.children() {
                    match child.name() {
                        Name::SpaceOrTab if last_event_was_code_flow_chunk => {
                            let range = child.byte_range();

                            // a `clean_code_chunk` may have already removed the whole line
                            if range.end <= out.rest().len() {
                                out.remove(range)
                            }
                        }
                        Name::CodeFlowChunk => {
                            clean_code_chunk(&mut out, markdown, child.byte_range());
                        }
                        _ => (),
                    }

                    last_event_was_code_flow_chunk = child.name() == Name::CodeFlowChunk;
                }

                out.insert(range.start, "```rust\n");
            }
            Name::Link => {
                let Some(label) = node.child(Name::Label) else {
                    continue;
                };

                if let Some(resource) = node.child(Name::Resource) {
                    let Some(dest) = resource.child(Name::ResourceDestination) else {
                        continue;
                    };

                    let Some(dest_string) = dest.descendant(Name::ResourceDestinationString) else {
                        continue;
                    };

                    let Some(&resolved) = links.get(dest_string.str()) else {
                        continue;
                    };

                    let Some(new_url) = resolved else {
                        let Some(label_text) = label.child(Name::LabelText) else {
                            continue;
                        };

                        out.replace(node.byte_range(), label_text.str());
                        continue;
                    };

                    out.replace(dest.byte_range(), new_url);
                    // TODO: correctly escape / add angled brackets
                    continue;
                }

                if let Some(reference) = node.child(Name::Reference) {
                    let identifier = match reference.child(Name::ReferenceString) {
                        Some(string) => string.str(),
                        None => match label.child(Name::LabelText) {
                            Some(label_text) => label_text.str(),
                            None => continue,
                        },
                    };

                    let Some(&resolved) = links.get(identifier) else {
                        continue;
                    };

                    let Some(new_url) = resolved else {
                        let Some(label_text) = label.child(Name::LabelText) else {
                            continue;
                        };

                        out.replace(node.byte_range(), label_text.str());
                        continue;
                    };

                    // refers to a definition
                    _ = new_url;
                    continue;
                }

                // shortcut
                let Some(label_text) = label.child(Name::LabelText) else {
                    continue;
                };

                let Some(&resolved) = links.get(label_text.str()) else {
                    continue;
                };

                let Some(new_url) = resolved else {
                    let Some(label_text) = label.child(Name::LabelText) else {
                        continue;
                    };

                    out.replace(node.byte_range(), label_text.str());
                    continue;
                };

                // refers to a definition
                _ = new_url;
            }
            Name::Definition => {
                let Some(dest) = node.child(Name::DefinitionDestination) else {
                    continue;
                };

                let Some(dest_string) = dest.descendant(Name::DefinitionDestinationString) else {
                    continue;
                };

                let Some(label) = node.descendant(Name::DefinitionLabelString) else {
                    continue;
                };

                if dest_string.str() == PLACEHOLDER_DESTINATION
                    || unused_definitions.contains(label.str())
                {
                    let mut range = node.byte_range();
                    range.end = end_of_line(markdown, range.end);
                    out.remove(range);
                    continue;
                }

                let Some(&resolved) = links.get(dest_string.str()) else {
                    continue;
                };

                let Some(new_url) = resolved else {
                    let mut range = node.byte_range();
                    range.end = end_of_line(markdown, range.end);
                    out.remove(range);
                    continue;
                };

                out.replace(dest.byte_range(), new_url);
                // TODO: correctly escape / add angled brackets
            }
            _ => (),
        }
    }

    out.finish()
}

struct Tree<'m, 'e> {
    markdown: &'m str,
    events: &'e [Event],
}

impl<'m, 'e> Tree<'m, 'e> {
    fn depth_first(&self) -> impl Iterator<Item = TreeAt<'m, 'e, '_>> {
        (0..self.events.len())
            .rev()
            .filter(|&index| self.events[index].kind == Kind::Exit)
            .map(|index| TreeAt { tree: self, index })
    }
}

#[derive(Clone, Copy)]
struct TreeAt<'m, 'e, 't> {
    tree: &'t Tree<'m, 'e>,
    index: usize,
}

impl<'m, 'e, 't> TreeAt<'m, 'e, 't> {
    fn name(&self) -> Name {
        self.tree.events[self.index].name.clone()
    }

    fn str(&self) -> &'m str {
        &self.tree.markdown[self.byte_range()]
    }

    fn child(self, name: Name) -> Option<TreeAt<'m, 'e, 't>> {
        child(self.tree.events, self.index, name).map(|index| TreeAt { tree: self.tree, index })
    }

    fn children(self) -> impl Iterator<Item = TreeAt<'m, 'e, 't>> {
        children(self.tree.events, self.index).map(|index| TreeAt { tree: self.tree, index })
    }

    fn descendant(self, name: Name) -> Option<TreeAt<'m, 'e, 't>> {
        descendant(self.tree.events, self.index, name)
            .map(|index| TreeAt { tree: self.tree, index })
    }

    fn descendants_with_name(self, name: Name) -> impl Iterator<Item = TreeAt<'m, 'e, 't>> {
        descendants_with_name(self.tree.events, self.index, name)
            .map(|index| TreeAt { tree: self.tree, index })
    }

    fn byte_range(self) -> Range<usize> {
        byte_range(self.tree.events, self.index)
    }
}

fn unused_definitions<'a>(
    markdown: &'a str,
    events: &[Event],
    options: &'a RewriteMarkdownOptions,
) -> HashSet<&'a str> {
    let mut used_definitions: HashSet<&str> = HashSet::new();
    let tree = Tree { events, markdown };

    for node in tree.depth_first() {
        if node.name() != Name::Link {
            continue;
        }

        if node.descendant(Name::Resource).is_some() {
            continue;
        }

        let identifier = match node.descendant(Name::ReferenceString) {
            Some(some) => some,
            None => match node.descendant(Name::LabelText) {
                Some(some) => some,
                None => continue,
            },
        };

        used_definitions.insert(identifier.str());
    }

    let all_definitions: HashSet<&str> = options.links.iter().map(|(k, _)| k.as_str()).collect();
    all_definitions.difference(&used_definitions).copied().collect()
}

fn start_of_line(markdown: &str, index: usize) -> usize {
    match markdown[..index].bytes().rposition(|b| b == b'\n') {
        Some(i) => i + 1,
        None => 0,
    }
}

fn end_of_line(markdown: &str, index: usize) -> usize {
    match markdown[index..].bytes().position(|b| b == b'\n') {
        Some(i) => index + i + 1,
        None => markdown.len(),
    }
}

fn expand_to_line(markdown: &str, range: Range<usize>) -> Range<usize> {
    start_of_line(markdown, range.start)..end_of_line(markdown, range.end)
}

fn clean_code_chunk(out: &mut StringReplacer, markdown: &str, range: Range<usize>) {
    let line = &markdown[range.clone()];
    let line_trim_start = line.trim_start();

    if let Some(rest) = line_trim_start.strip_prefix('#') {
        match rest.bytes().next() {
            Some(b' ') | None => {
                out.remove(expand_to_line(markdown, range));
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
