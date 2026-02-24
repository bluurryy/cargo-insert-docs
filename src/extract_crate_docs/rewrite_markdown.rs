#[cfg(test)]
mod tests;

use core::{fmt::Write, ops::Range};
use std::collections::{HashMap, HashSet};

use crate::{
    markdown::{self, Tree, format_link_destination},
    markdown_rs::event::Name,
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
        let destination =
            format_link_destination(destination.as_deref().unwrap_or(PLACEHOLDER_DESTINATION));
        markdown.write_fmt(format_args!("[{identifier}]: {destination}\n")).unwrap();
    }

    markdown
}

fn rewrite(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    let links: HashMap<&str, Option<&str>> =
        options.links.iter().map(|(k, v)| (k.as_str(), v.as_deref())).collect();

    let tree = Tree::new(markdown);

    if tree.events.is_empty() {
        return markdown.into();
    }

    let mut out = StringReplacer::new(markdown);
    let unused_definitions = unused_definitions(&tree, options);

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

                    out.replace(fence_info.byte_range(), "rust");

                    for child in node.children_with_name(Name::CodeFlowChunk) {
                        clean_code_chunk(&mut out, markdown, child.byte_range());
                    }
                } else if let Some(fence) = node.descendant(Name::CodeFencedFenceSequence) {
                    out.insert(fence.byte_range().end, "rust");

                    for child in node.children_with_name(Name::CodeFlowChunk) {
                        clean_code_chunk(&mut out, markdown, child.byte_range());
                    }
                }
            }
            Name::CodeIndented => {
                let range = node.byte_range();
                let mut last_space = None;

                out.insert(range.start, "```rust\n");

                for child in node.children() {
                    match child.name() {
                        Name::SpaceOrTab => {
                            last_space = Some(child);
                        }
                        Name::CodeFlowChunk => {
                            let space =
                                last_space.expect("an indented codeblock must be indented (duh)");

                            match clean_code_line(child.str()) {
                                Some(CleanAction::RemoveLine) => {
                                    out.remove(expand_to_line(markdown, child.byte_range()));
                                }
                                Some(CleanAction::RemoveHash(idx)) => {
                                    let hash = idx + child.byte_range().start;
                                    out.remove(space.byte_range());
                                    out.remove(hash..hash + 1);
                                }
                                None => {
                                    out.remove(space.byte_range());
                                }
                            }
                        }
                        _ => (),
                    }
                }

                out.insert(range.end, "\n```");
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

                    out.replace(dest.byte_range(), format_link_destination(new_url));
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

                out.replace(dest.byte_range(), format_link_destination(new_url));
            }
            _ => (),
        }
    }

    out.finish()
}

fn unused_definitions<'a>(
    tree: &Tree<'a>,
    options: &'a RewriteMarkdownOptions,
) -> HashSet<&'a str> {
    let mut used_definitions: HashSet<&str> = HashSet::new();

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
    match clean_code_line(&markdown[range.clone()]) {
        Some(CleanAction::RemoveLine) => {
            out.remove(expand_to_line(markdown, range.clone()));
        }
        Some(CleanAction::RemoveHash(idx)) => {
            let hash = idx + range.start;
            out.remove(hash..hash + 1);
        }
        None => (),
    }
}

fn clean_code_line(line: &str) -> Option<CleanAction> {
    let line_trim_start = line.trim_start();

    if let Some(rest) = line_trim_start.strip_prefix('#') {
        match rest.bytes().next() {
            Some(b' ') | None => return Some(CleanAction::RemoveLine),
            Some(b'#') => {
                // double hash `##`, remove one of the hashes
                let idx = substr_range(line, line_trim_start).start;
                return Some(CleanAction::RemoveHash(idx));
            }
            Some(_) => (),
        }
    }

    None
}

pub enum CleanAction {
    RemoveLine,
    RemoveHash(usize),
}

fn substr_range(str: &str, substr: &str) -> Range<usize> {
    let start = substr.as_ptr() as usize - str.as_ptr() as usize;
    let end = start + substr.len();
    start..end
}

fn code_block_fence_is_rust(lang: &str) -> bool {
    match markdown::lang_string::is_rust(lang) {
        Ok(is_rust) => is_rust,
        Err(errors) => {
            let _span = errors
                .into_iter()
                .map(|error| tracing::warn_span!("", error).entered())
                .collect::<Vec<_>>();

            tracing::warn!("failed to parse code block language");
            false
        }
    }
}
