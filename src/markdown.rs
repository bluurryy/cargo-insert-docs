#[cfg(test)]
mod tests;

use std::{borrow::Cow, ops::Range};

use color_eyre::eyre::{self, bail};
use pulldown_cmark::{
    BrokenLink, BrokenLinkCallback, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType,
    OffsetIter, Options, Parser, Tag, TagEnd,
};

use crate::string_replacer::StringReplacer;

pub fn parser<'a>(text: &'a str) -> OffsetIter<'a, impl BrokenLinkCallback<'a>> {
    // Interprets `[Vec]` as `[Vec](Vec)` which is fine for rust docs.
    fn broken_link_callback<'a>(broken_link: BrokenLink<'a>) -> Option<(CowStr<'a>, CowStr<'a>)> {
        Some((broken_link.reference, CowStr::Borrowed("")))
    }

    parser_with_broken_link_callback(text, broken_link_callback)
}

/// The same parser rustdoc uses.
pub fn parser_with_broken_link_callback<'a, F>(text: &'a str, callback: F) -> OffsetIter<'a, F>
where
    F: BrokenLinkCallback<'a>,
{
    // The same options rustdoc uses.
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;

    Parser::new_with_broken_link_callback(text, options, Some(callback)).into_offset_iter()
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

#[derive(Debug)]
pub struct Link<'a> {
    pub span: Range<usize>,
    pub link_type: LinkType,
    pub dest_url: CowStr<'a>,
    pub title: CowStr<'a>,
    #[expect(dead_code)]
    pub id: CowStr<'a>,
    pub content_span: Option<Range<usize>>,
}

pub fn links<'a>(markdown: &'a str) -> Vec<Link<'a>> {
    let mut links = vec![];
    let mut stack = vec![];

    for (event, span) in parser(markdown) {
        match event {
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                stack.push(Link { span, link_type, dest_url, title, id, content_span: None })
            }
            Event::End(TagEnd::Link) => links.push(stack.pop().unwrap()),
            _ => {
                if let Some(link) = stack.last_mut() {
                    add_span(&mut link.content_span, span);
                }
            }
        }
    }

    links
}

fn add_span(span: &mut Option<Range<usize>>, additional: Range<usize>) {
    let span = span.get_or_insert(additional.clone());
    span.start = span.start.min(additional.start);
    span.end = span.end.max(additional.end);
}

struct CodeBlock<'a> {
    span: Range<usize>,
    kind: CodeBlockKind<'a>,
    content: Option<Range<usize>>,
}

pub fn clean_code_blocks(markdown: &str) -> String {
    let mut out = StringReplacer::new(markdown);

    for block in code_blocks(markdown).into_iter().rev() {
        match &block.kind {
            CodeBlockKind::Indented => {
                // all indented code blocks are considered rust
            }
            CodeBlockKind::Fenced(info) => {
                if !fenced_code_block_is_rust(info) {
                    continue;
                }
            }
        }

        if let Some(content) = block.content.clone() {
            if matches!(block.kind, CodeBlockKind::Indented) {
                let mut new_content = String::new();

                for mut line in markdown[block.span.clone()].lines() {
                    // empty lines are allowed to not be prefixed by four spaces
                    line = line.strip_prefix("    ").unwrap_or(line);

                    if let Some(line) = clean_code_line(line) {
                        new_content.push_str(&line);
                        new_content.push('\n');
                    }
                }

                new_content = format!("```rust\n{new_content}```\n");

                out.replace(block.span.clone(), new_content);
            } else {
                let mut new_content = String::new();

                for line in markdown[content.clone()].lines() {
                    if let Some(line) = clean_code_line(line) {
                        new_content.push_str(&line);
                        new_content.push('\n');
                    }
                }

                out.replace(content, new_content);
            }
        }

        if let CodeBlockKind::Fenced(_) = &block.kind {
            let line = markdown[block.span.start + 3..]
                .lines()
                .next()
                .expect("it wouldn't be a code block ");
            let info_span = substr_range(markdown, line);
            out.replace(info_span, "rust");
        }
    }

    out.finish()
}

// remove hidden lines
// <https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html#hiding-portions-of-the-example>
fn clean_code_line(line: &str) -> Option<Cow<'_, str>> {
    let line_trim_start = line.trim_start();

    if let Some(rest) = line_trim_start.strip_prefix('#') {
        match rest.bytes().next() {
            Some(b' ') | None => None,
            Some(b'#') => {
                let mid = substr_range(line, line_trim_start).start;
                let lhs = &line[..mid];
                let rhs = &line[mid + 1..];
                Some(format!("{lhs}{rhs}").into())
            }
            Some(_) => Some(Cow::Borrowed(line)),
        }
    } else {
        Some(Cow::Borrowed(line))
    }
}

fn substr_range(str: &str, substr: &str) -> Range<usize> {
    let start = substr.as_ptr() as usize - str.as_ptr() as usize;
    let end = start + substr.len();
    start..end
}

fn code_blocks<'a>(markdown: &'a str) -> Vec<CodeBlock<'a>> {
    let mut blocks = vec![];
    let mut stack = vec![];

    for (event, span) in parser(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => stack.push(CodeBlock {
                span: if matches!(kind, CodeBlockKind::Indented) {
                    // make the span start at the start of the line
                    markdown[..span.start].rfind('\n').unwrap_or(usize::MAX).wrapping_add(1)
                        ..span.end
                } else {
                    span
                },
                kind,
                content: None,
            }),
            Event::End(TagEnd::CodeBlock) => blocks.push(stack.pop().unwrap()),
            _ => {
                if let Some(block) = stack.last_mut() {
                    add_span(&mut block.content, span);
                }
            }
        }
    }

    blocks
}

fn fenced_code_block_is_rust(name: &str) -> bool {
    const STARTS: &[&str] = &[
        "rust",
        "ignore",
        "should_panic",
        "no_run",
        "compile_fail",
        "edition",
        "standalone_crate",
    ];

    if name.is_empty() {
        return true;
    }

    for start in STARTS {
        if name.starts_with(start) {
            return true;
        }
    }

    false
}

pub fn shrink_headings(markdown: &str) -> String {
    let mut out = StringReplacer::new(markdown);

    for heading in headings(markdown).into_iter().rev() {
        out.insert(heading.span.start, "#");
    }

    out.finish()
}

struct Heading {
    span: Range<usize>,
    #[expect(dead_code)]
    level: HeadingLevel,
}

fn headings(markdown: &str) -> Vec<Heading> {
    let mut headings = vec![];

    for (event, span) in parser(markdown) {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            headings.push(Heading { span, level })
        }
    }

    headings
}

pub fn rewrite_link_definition_urls(
    markdown: &str,
    mut rewrite: impl FnMut(&str) -> Option<String>,
) -> String {
    use ::markdown::{ParseOptions, mdast, to_mdast};

    let options = ParseOptions::gfm();
    let mut node = to_mdast(markdown, &options).expect("non mdx parsing can't fail");
    let mut out = StringReplacer::new(markdown);

    fn rec(
        out: &mut StringReplacer,
        node: &mut mdast::Node,
        rewrite: &mut impl FnMut(&str) -> Option<String>,
    ) {
        if let Some(children) = node.children_mut() {
            for child in children.iter_mut().rev() {
                rec(out, child, rewrite)
            }
        } else if let mdast::Node::Definition(def) = node {
            let pos = def.position.as_ref().expect("position should be set");
            let start = pos.start.offset;
            let end = pos.end.offset;

            if let Some(new_url) = rewrite(&def.url) {
                def.url = new_url;
                // can't fail unless a bad quote character was chosen in the serialization options
                let new_str = mdast_util_to_markdown::to_markdown(node)
                    .expect("serializing a definition should not fail");
                out.replace(start..end, new_str);
            }
        }
    }

    rec(&mut out, &mut node, &mut rewrite);
    out.finish()
}
