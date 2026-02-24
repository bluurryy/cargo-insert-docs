use std::ops::Range;

use color_eyre::eyre::{self, bail};

use crate::{markdown::Tree, markdown_rs::event::Name};

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

    let is_start = |s| parts_eq(s, &["<!-- ", section_name, " start -->"]);
    let is_end = |s| parts_eq(s, &["<!-- ", section_name, " end -->"]);

    let mut start = None::<Range<usize>>;

    for comment in find_html_comments(markdown) {
        let comment_str = &markdown[comment.clone()];

        if let Some(start) = start.clone() {
            if is_end(comment_str) {
                return Some(Section {
                    span: start.start..comment.end,
                    content_span: start.end..comment.start,
                });
            }
        } else if is_start(comment_str) {
            start = Some(comment);
        }
    }

    None
}

#[derive(Debug)]
pub struct Section {
    pub span: Range<usize>,
    pub content_span: Range<usize>,
}

/// Finds subsections like these:
/// ```md
/// <!-- section_name foo start -->
/// This is the content of the "foo" subsection.
/// <!-- section_name foo end -->
///
/// <!-- section_name bar start -->
/// This is the content of the "bar" subsection.
/// <!-- section_name bar end -->
/// ```
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
    find_html(markdown).flat_map(|html| {
        comments(&markdown[html.clone()])
            .map(move |comment| comment.start + html.start..comment.end + html.start)
    })
}

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

fn find_html(markdown: &str) -> impl Iterator<Item = Range<usize>> {
    let tree = Tree::new(markdown);

    // We don't use `Tree::depth_first` because of borrow issues.
    (0..tree.events.len()).filter_map(move |index| {
        let node = tree.at(index)?;

        if !matches!(node.name(), Name::HtmlFlow | Name::HtmlText) {
            return None;
        }

        Some(node.byte_range())
    })
}
