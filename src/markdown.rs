pub mod lang_string;
mod section;
#[cfg(test)]
mod tests;
mod tree;

use std::borrow::Cow;

use percent_encoding::percent_encode_byte;

use crate::{markdown_rs::event::Name, string_replacer::StringReplacer};

pub use section::{find_section, find_subsections};
pub use tree::Tree;

pub fn extract_definitions(markdown: &str) -> [String; 2] {
    let mut out = StringReplacer::new(markdown);
    let mut definitions: Vec<&str> = vec![];
    let tree = Tree::new(markdown);

    for node in tree.depth_first() {
        if node.name() == Name::Definition {
            let mut range = node.byte_range();
            range.end = end_of_line(markdown, range.end);
            let value = &markdown[range.clone()];
            definitions.push(value);
            out.remove(range);
        }
    }

    let without_definitions = out.finish();
    // FIXME: remove the rev, just for testing
    let definitions = definitions.into_iter().rev().collect::<Vec<_>>().join("");

    [without_definitions, definitions]
}

pub fn end_of_line(markdown: &str, index: usize) -> usize {
    match markdown[index..].bytes().position(|b| b == b'\n') {
        Some(i) => index + i + 1,
        None => markdown.len(),
    }
}

pub fn format_link_destination(destination: &str) -> Cow<'_, str> {
    let needs_angle_brackets = destination.is_empty()
        || destination.starts_with('<')
        || destination
            .chars()
            .any(|c| c.is_ascii_whitespace() || c.is_ascii_control() || c == '(' || c == ')');

    if !needs_angle_brackets {
        return Cow::Borrowed(destination);
    }

    let mut out = String::new();
    out.push('<');

    for char in destination.chars() {
        if matches!(char, '\n' | '<' | '>') {
            out.push_str(percent_encode_byte(char as u8));
        } else {
            out.push(char);
        }
    }

    out.push('>');
    Cow::Owned(out)
}
