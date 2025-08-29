#[cfg(test)]
mod tests;

use std::ops::Range;

use color_eyre::eyre::{self, bail};
use pulldown_cmark::{BrokenLink, BrokenLinkCallback, CowStr, Event, OffsetIter, Options, Parser};

use crate::{
    markdown_rs::{
        self, ParseOptions,
        event::{Kind, Name},
        parser::ParseState,
        unist::Position,
    },
    string_replacer::StringReplacer,
};

/// The same parser rustdoc uses.
pub fn parser<'a>(text: &'a str) -> OffsetIter<'a, impl BrokenLinkCallback<'a>> {
    // Interprets `[Vec]` as `[Vec](Vec)` which is fine for rust docs.
    fn broken_link_callback<'a>(broken_link: BrokenLink<'a>) -> Option<(CowStr<'a>, CowStr<'a>)> {
        Some((broken_link.reference, CowStr::Borrowed("")))
    }

    // The same options rustdoc uses.
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;

    Parser::new_with_broken_link_callback(text, options, Some(broken_link_callback))
        .into_offset_iter()
}

/// Finds sections like these:
/// ```md
/// <!-- section_name start -->
/// This is the section content.
/// <!-- section_name end -->
/// ```
pub fn find_section(markdown: &str, section_name: &str) -> Option<Section> {
    let start_marker = format!("<!-- {section_name} start -->");
    let end_marker = format!("<!-- {section_name} end -->");

    let mut start = None::<Range<usize>>;

    for comment in find_html_comments(markdown) {
        if let Some(start) = start.clone() {
            if markdown[comment.clone()] == end_marker {
                return Some(Section {
                    span: start.start..comment.end,
                    content_span: start.end..comment.start,
                });
            }
        } else if markdown[comment.clone()] == start_marker {
            start = Some(comment);
        }
    }

    None
}

pub struct Section {
    pub span: Range<usize>,
    pub content_span: Range<usize>,
}

pub fn find_subsections<'a>(
    markdown: &'a str,
    section_name: &str,
) -> eyre::Result<Vec<(Section, &'a str)>> {
    let mut sections = vec![];

    let mut start = None::<(Range<usize>, &'a str)>;

    for (range, kind, name) in find_subsection_tags(markdown, section_name) {
        if let Some((start_range, start_name)) = start {
            if name == start_name && kind == SectionTagKind::End {
                sections.push((
                    Section {
                        span: start_range.start..range.end,
                        content_span: start_range.end..range.start,
                    },
                    name,
                ));
                start = None;
            } else {
                bail!("subsections must be disjoint");
            }
        } else {
            if kind == SectionTagKind::Start {
                start = Some((range, name));
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

        let mut end = 0;

        std::iter::from_fn(move || {
            let start = html[end..].find(START)? + end;
            end = html[start..].find(END)? + start + END.len();
            Some(start..end)
        })
    }

    find_html(markdown).flat_map(|html| {
        comments(&markdown[html.clone()])
            .map(move |comment| comment.start + html.start..comment.end + html.start)
    })
}

fn find_html(markdown: &str) -> impl Iterator<Item = Range<usize>> {
    parser(markdown).filter_map(|(event, range)| {
        matches!(event, Event::Html(_) | Event::InlineHtml(_)).then_some(range)
    })
}

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

pub fn extract_definitions(markdown: &str) -> [String; 2] {
    let mut out = StringReplacer::new(markdown);
    let (events, _state) = parse(markdown, &parse_options());
    let events = events.as_slice();
    let mut definitions: Vec<&str> = vec![];

    for index in (0..events.len()).rev() {
        let event = &events[index];

        if event.kind == Kind::Enter {
            continue;
        }

        if event.name == Name::Definition {
            let mut range = byte_range(events, index);
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

pub fn byte_range(events: &[markdown_rs::event::Event], index: usize) -> Range<usize> {
    let pos = position(events, index);
    pos.start.offset..pos.end.offset
}

fn position(events: &[markdown_rs::event::Event], exit_index: usize) -> Position {
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

pub fn end_of_line(markdown: &str, index: usize) -> usize {
    match markdown[index..].bytes().position(|b| b == b'\n') {
        Some(i) => index + i + 1,
        None => markdown.len(),
    }
}
