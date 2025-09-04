#[cfg(test)]
mod tests;

use core::{mem::take, ops::Range};
use std::collections::{HashMap, HashSet};

use crate::{
    markdown_rs::util::normalize_identifier::normalize_identifier, string_replacer::StringReplacer,
};

#[derive(Default)]
pub struct RewriteMarkdownOptions {
    pub shrink_headings: i8,
    pub links: Vec<(String, Option<String>)>,
}

pub fn rewrite_markdown(markdown: &str, options: &RewriteMarkdownOptions) -> String {
    use ::markdown::mdast::{Definition, Node, Root};

    fn to_markdown(node: &Node) -> String {
        let options = mdast_util_to_markdown::Options {
            bullet: '-',
            bullet_other: '*',
            ..Default::default()
        };
        mdast_util_to_markdown::to_markdown_with_options(node, &options)
            .expect("should only fail with bad options")
    }

    let parse_options = ::markdown::ParseOptions::gfm();

    // add definitions, so our broken references actually get parsed as references
    let mut markdown = markdown.to_string();

    if !options.links.is_empty() {
        markdown.push_str("\n\n");
    }

    markdown.push_str(&to_markdown(&Node::Root(Root {
        position: None,
        children: options
            .links
            .iter()
            .map(|(identifier, destination)| {
                Node::Definition(Definition {
                    position: None,
                    url: destination.as_deref().unwrap_or(PLACEHOLDER_DESTINATION).into(),
                    title: None,
                    identifier: identifier.into(),
                    label: None,
                })
            })
            .collect(),
    })));

    let mut node = ::markdown::to_mdast(&markdown, &parse_options)
        .expect("should only fail for mdx which we don't enable");

    fn used_definitions(node: &Node) -> IdentifierSet {
        let mut used_definitions: HashSet<String> = HashSet::new();

        fn walk(node: &Node, used_definitions: &mut HashSet<String>) {
            match node {
                Node::LinkReference(link_ref) => {
                    used_definitions.insert(link_ref.identifier.clone());
                }
                _ => {
                    if let Some(children) = node.children() {
                        for child in children {
                            walk(child, used_definitions);
                        }
                    }
                }
            }
        }

        walk(node, &mut used_definitions);
        IdentifierSet::new(used_definitions)
    }

    enum WalkResult {
        Remove,
        Keep,
        Dissolve,
    }

    fn walk(node: &mut Node, cx: &mut Cx) -> WalkResult {
        use WalkResult::{Dissolve, Keep, Remove};

        match node {
            Node::Heading(heading) => {
                heading.depth = (heading.depth as i8)
                    .saturating_add(cx.options.shrink_headings)
                    .clamp(1, 6) as u8;

                Keep
            }
            Node::Code(code) => {
                if code.lang.as_deref().is_none_or(code_block_fence_is_rust) {
                    code.lang = Some("rust".into());

                    // this newline will be removed at the end of this block;
                    // we add it here to make line removal of the last line
                    // play nice if it doesn't end in a newline itself
                    code.value.push('\n');

                    let mut out = StringReplacer::new(&code.value);

                    for line in code.value.lines().rev() {
                        let range = substr_range(&code.value, line);
                        clean_code_chunk(&mut out, &code.value, range);
                    }

                    code.value = out.finish();

                    // remove the newline we added
                    if code.value.ends_with('\n') {
                        code.value.pop();
                    }
                }

                Keep
            }
            Node::Link(link) => {
                let Some(resolved) = cx.links.get(link.url.as_str()) else {
                    return Keep;
                };

                let Some(new_url) = resolved else {
                    return Dissolve;
                };

                link.url = new_url.to_string();
                Keep
            }
            Node::LinkReference(link_ref) => {
                let Some(resolved) = cx.links.get(link_ref.identifier.as_str()) else {
                    return Keep;
                };

                if resolved.is_none() {
                    return Dissolve;
                };

                Keep
            }
            Node::Definition(def) => {
                if def.url == PLACEHOLDER_DESTINATION {
                    return Remove;
                }

                if cx.unused_definitions.contains(&def.identifier) {
                    return Remove;
                }

                let Some(resolved) = cx.links.get(def.url.as_str()) else {
                    return Keep;
                };

                let Some(new_url) = resolved else {
                    return Remove;
                };

                def.url = new_url.to_string();
                Keep
            }
            _ => {
                if let Some(children) = node.children_mut() {
                    let mut new_children = vec![];

                    for mut child in take(children) {
                        match walk(&mut child, cx) {
                            Remove => (),
                            Keep => new_children.push(child),
                            Dissolve => {
                                if let Some(descendants) = child.children_mut() {
                                    new_children.extend(take(descendants));
                                }
                            }
                        }
                    }

                    *children = new_children;
                }

                Keep
            }
        }
    }

    struct Cx<'a> {
        options: &'a RewriteMarkdownOptions,
        unused_definitions: IdentifierSet,
        links: IdentifierMap<Option<&'a str>>,
    }

    let mut cx = Cx {
        options,
        unused_definitions: {
            let used = used_definitions(&node);
            let all = IdentifierSet::new(options.links.iter().map(|(k, _)| k));
            all.difference(&used)
        },
        links: IdentifierMap::new(options.links.iter().map(|(k, v)| (k, v.as_deref()))),
    };

    walk(&mut node, &mut cx);

    to_markdown(&node)
}

#[derive(Debug)]
struct IdentifierSet(HashSet<String>);

impl IdentifierSet {
    fn new(set: impl IntoIterator<Item: AsRef<str>>) -> Self {
        Self(set.into_iter().map(|s| normalize_identifier(s.as_ref())).collect())
    }

    fn contains(&self, id: &str) -> bool {
        self.0.contains(&normalize_identifier(id))
    }

    fn difference(&self, other: &IdentifierSet) -> IdentifierSet {
        Self(self.0.difference(&other.0).cloned().collect())
    }
}

#[derive(Debug)]
struct IdentifierMap<V>(HashMap<String, V>);

impl<V> IdentifierMap<V> {
    fn new<K: AsRef<str>>(set: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(set.into_iter().map(|(k, v)| (normalize_identifier(k.as_ref()), v)).collect())
    }

    fn get(&self, id: &str) -> Option<&V> {
        self.0.get(&normalize_identifier(id))
    }
}

/// If we were not able to resolve an item, then it will get this placeholder destination.
/// A definition with a placeholder destination will be removed from the final markdown.
/// We need to temporarily create a definition with a placeholder destination, so
/// these references actually parse as references. Those references will then get
/// replaced with their label only.
const PLACEHOLDER_DESTINATION: &str = "__PLACEHOLDER_DESTINATION__";

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
