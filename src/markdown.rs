#[cfg(test)]
mod tests;

use std::ops::Range;

use color_eyre::eyre::{self, bail};

use crate::{
    markdown_rs::{
        self, ParseOptions,
        event::{Kind, Name},
        parser::ParseState,
        unist::Position,
    },
    string_replacer::StringReplacer,
};

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
    let (events, _state) = parse(markdown, &parse_options());

    (0..events.len()).rev().filter_map(move |index| {
        let event = &events[index];

        if event.kind == Kind::Enter {
            return None;
        }

        if !matches!(event.name, Name::HtmlFlow | Name::HtmlText) {
            return None;
        }

        Some(byte_range(&events, index))
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

/// Finds sections like these:
/// ```md
/// <!-- section_name start -->
/// This is the section content.
/// <!-- section_name end -->
/// ```
pub fn find_section(markdown: &str, section_name: &str) -> Option<Section> {
    let start_marker = format!("<!-- {section_name} start -->");
    let end_marker = format!("<!-- {section_name} end -->");

    let (events, _state) = parse(markdown, &parse_options());
    let events = events.as_slice();
    let mut end = None::<Range<usize>>;

    for index in (0..events.len()).rev() {
        let event = &events[index];

        if event.kind == Kind::Enter {
            continue;
        }

        if !matches!(event.name, Name::HtmlFlow | Name::HtmlText) {
            continue;
        }

        let html = byte_range(events, index);
        let html_str = &markdown[html.clone()];

        if let Some(end) = end.clone() {
            if html_str == start_marker {
                return Some(Section {
                    span: html.start..end.end,
                    content_span: html.end..end.start,
                });
            }
        } else if html_str == end_marker {
            end = Some(html);
        }
    }

    None
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

pub fn byte_range(events: &[markdown_rs::event::Event], exit_index: usize) -> Range<usize> {
    let pos = position(events, exit_index);
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
