pub mod lang_string;
#[cfg(test)]
mod tests;

use std::{borrow::Cow, ops::Range};

use color_eyre::eyre::{self, bail};
use percent_encoding::percent_encode_byte;

use crate::{
    markdown_rs::{
        self, ParseOptions,
        event::{Event, Kind, Name},
        parser::ParseState,
        unist::Position,
    },
    string_replacer::StringReplacer,
};

pub fn parse<'a>(
    markdown: &'a str,
    parse_options: &'a ParseOptions,
) -> (Vec<markdown_rs::event::Event>, ParseState<'a>) {
    markdown_rs::parser::parse(markdown, parse_options)
        .expect("should only fail for mdx which we don't enable")
}

pub fn parse_options() -> ParseOptions {
    markdown_rs::ParseOptions::gfm()
}

/// Finds sections like these:
/// ```md
/// <!-- section_name start -->
/// This is the section content.
/// <!-- section_name end -->
/// ```
pub fn find_section(markdown: &str, section_name: &str) -> Option<Section> {
    fn parts_eq(mut str: &str, parts: &[&str]) -> bool {
        for &part in parts {
            str = match str.strip_prefix(part) {
                Some(rest) => rest,
                None => return false,
            }
        }

        str.is_empty()
    }

    let is_end = |s| parts_eq(s, &["<!-- ", section_name, " end -->"]);
    let is_start = |s| parts_eq(s, &["<!-- ", section_name, " start -->"]);

    let mut end = None::<Range<usize>>;

    for comment in find_html_comments(markdown) {
        let comment_str = &markdown[comment.clone()];

        if let Some(end) = end.clone() {
            if is_start(comment_str) {
                return Some(Section {
                    span: comment.start..end.end,
                    content_span: comment.end..end.start,
                });
            }
        } else if is_end(comment_str) {
            end = Some(comment);
        }
    }

    None
}

#[derive(Debug)]
pub struct Section {
    pub span: Range<usize>,
    pub content_span: Range<usize>,
}

pub fn find_subsections<'a>(
    markdown: &'a str,
    section_name: &str,
) -> eyre::Result<Vec<(Section, &'a str)>> {
    let mut sections = vec![];

    let mut end = None::<(Range<usize>, &'a str)>;

    for (range, kind, name) in find_subsection_tags(markdown, section_name) {
        if let Some((end_range, end_name)) = end {
            if name == end_name && kind == SectionTagKind::Start {
                sections.push((
                    Section {
                        span: range.start..end_range.end,
                        content_span: range.end..end_range.start,
                    },
                    name,
                ));
                end = None;
            } else {
                bail!("subsections must be disjoint");
            }
        } else {
            if kind == SectionTagKind::End {
                end = Some((range, name));
            } else {
                bail!("subsection end without start");
            }
        }
    }

    Ok(sections)
}

fn find_subsection_tags<'a>(
    markdown: &'a str,
    section_name: &str,
) -> impl Iterator<Item = (Range<usize>, SectionTagKind, &'a str)> {
    fn parse_name_and_kind(str: &str) -> Option<(&str, SectionTagKind)> {
        if let Some(name) = str.strip_suffix(" start") {
            return Some((name, SectionTagKind::Start));
        }

        if let Some(name) = str.strip_suffix(" end") {
            return Some((name, SectionTagKind::End));
        }

        None
    }

    find_html_comments(markdown).filter_map(move |comment| {
        let name_and_kind = markdown[comment.clone()]
            .strip_prefix("<!-- ")?
            .strip_suffix(" -->")?
            .strip_prefix(section_name)?
            .strip_prefix(' ')?;

        let (name, kind) = parse_name_and_kind(name_and_kind)?;

        Some((comment, kind, name))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionTagKind {
    Start,
    End,
}

fn find_html_comments(markdown: &str) -> impl Iterator<Item = Range<usize>> {
    fn comments(html: &str) -> impl Iterator<Item = Range<usize>> {
        const START: &str = "<!--";
        const END: &str = "-->";

        let mut start = html.len();

        std::iter::from_fn(move || {
            let end = html[..start].rfind(END)? + END.len();
            start = html[..end].rfind(START)?;
            Some(start..end)
        })
    }

    find_html(markdown).flat_map(|html| {
        comments(&markdown[html.clone()])
            .map(move |comment| comment.start + html.start..comment.end + html.start)
    })
}

fn find_html(markdown: &str) -> impl Iterator<Item = Range<usize>> {
    let (events, _state) = parse(markdown, &parse_options());

    // We don't use `Tree::depth_first` because of borrow issues.
    (0..events.len()).rev().filter_map(move |index| {
        let event = &events[index];

        if event.kind != Kind::Exit {
            return None;
        }

        if !matches!(event.name, Name::HtmlFlow | Name::HtmlText) {
            return None;
        }

        let tree = Tree { markdown, events: &events };
        let node = tree.at(index).expect("event kind should be exit");

        Some(node.byte_range())
    })
}

pub fn extract_definitions(markdown: &str) -> [String; 2] {
    let mut out = StringReplacer::new(markdown);
    let (events, _state) = parse(markdown, &parse_options());
    let events = events.as_slice();
    let mut definitions: Vec<&str> = vec![];

    let tree = Tree { markdown, events };

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
    let definitions = definitions.join("");

    [without_definitions, definitions]
}

pub fn end_of_line(markdown: &str, index: usize) -> usize {
    match markdown[index..].bytes().position(|b| b == b'\n') {
        Some(i) => index + i + 1,
        None => markdown.len(),
    }
}

/// Tree interface to a slice of parser events.
///
/// All node iteration is done in reverse order to work nice with a [`StringReplacer`].
pub struct Tree<'m, 'e> {
    pub markdown: &'m str,
    pub events: &'e [Event],
}

impl<'m, 'e> Tree<'m, 'e> {
    /// Create a node from the given event index.
    ///
    /// A node must point at an `Exit` event.
    /// Returns `None` if the event is an `Enter` event.
    pub fn at(&self, index: usize) -> Option<Node<'m, 'e, '_>> {
        if self.events[index].kind == Kind::Enter { None } else { Some(Node { tree: self, index }) }
    }

    pub fn depth_first(&self) -> impl Iterator<Item = Node<'m, 'e, '_>> {
        (0..self.events.len()).rev().filter_map(|i| self.at(i))
    }
}

#[derive(Clone, Copy)]
pub struct Node<'m, 'e, 't> {
    tree: &'t Tree<'m, 'e>,
    index: usize,
}

impl<'m, 'e, 't> Node<'m, 'e, 't> {
    pub fn name(&self) -> Name {
        self.tree.events[self.index].name.clone()
    }

    pub fn str(&self) -> &'m str {
        &self.tree.markdown[self.byte_range()]
    }

    pub fn child(self, name: Name) -> Option<Self> {
        self.children_with_name(name).next()
    }

    pub fn children_with_name(self, name: Name) -> impl Iterator<Item = Self> {
        self.children().filter(move |n| n.name() == name)
    }

    pub fn children(self) -> impl Iterator<Item = Self> {
        let mut depth = 0;

        (0..self.index)
            .rev()
            .map_while(move |i| {
                let kind = self.tree.events[i].kind.clone();

                if depth == 0 && kind == Kind::Enter {
                    return None;
                }

                match kind {
                    Kind::Enter => depth -= 1,
                    Kind::Exit => depth += 1,
                }

                Some((i, depth))
            })
            .filter_map(|(i, depth)| (depth == 1).then_some(i))
            .filter_map(|index| self.tree.at(index))
    }

    pub fn descendant(self, name: Name) -> Option<Self> {
        self.descendants_with_name(name).next()
    }

    pub fn descendants_with_name(self, name: Name) -> impl Iterator<Item = Self> {
        self.descendants().filter(move |n| n.name() == name)
    }

    pub fn descendants(self) -> impl Iterator<Item = Self> {
        let mut depth = 0;

        (0..self.index)
            .rev()
            .take_while(move |&i| {
                let kind = self.tree.events[i].kind.clone();

                if depth == 0 && kind == Kind::Enter {
                    return false;
                }

                match kind {
                    Kind::Enter => depth -= 1,
                    Kind::Exit => depth += 1,
                }

                true
            })
            .filter_map(|i| self.tree.at(i))
    }

    pub fn byte_range(self) -> Range<usize> {
        let pos = self.position();
        pos.start.offset..pos.end.offset
    }

    pub fn position(self) -> Position {
        let event = &self.tree.events[self.index];
        let end = event.point.to_unist();
        let enter_index = self.enter_index();
        let start = self.tree.events[enter_index].point.to_unist();
        Position { start, end }
    }

    fn enter_index(self) -> usize {
        let mut depth = 0;

        for i in (0..self.index).rev() {
            let kind = self.tree.events[i].kind.clone();

            if depth == 0 && kind == Kind::Enter {
                return i;
            }

            match kind {
                Kind::Enter => depth -= 1,
                Kind::Exit => depth += 1,
            }
        }

        unreachable!("unpaired enter/exit event")
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
