#[cfg(test)]
mod tests;

use std::ops::Range;

use pulldown_cmark::{
    BrokenLink, BrokenLinkCallback, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType,
    OffsetIter, Options, Parser, Tag, TagEnd,
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
/// <!-- my_section start -->
/// This is the section content.
/// <!-- my_section end -->
/// ```
pub fn find_section(markdown: &str, section_name: &str) -> Option<Range<usize>> {
    let start_marker = format!("<!-- {section_name} start -->");
    let end_marker = format!("<!-- {section_name} end -->");

    let mut start = None;

    for (event, range) in parser(markdown) {
        if let Event::Html(html) | Event::InlineHtml(html) = event {
            if let Some(start) = start {
                if let Some(found) = html.find(&end_marker) {
                    let end = found + range.start;
                    return Some(start..end);
                }
            } else if let Some(found) = html.find(&start_marker) {
                start = Some(found + start_marker.len() + range.start);
            }
        }
    }

    None
}

#[derive(Debug)]
pub struct Link<'a> {
    pub span: Range<usize>,
    #[expect(dead_code)]
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
    let mut out = markdown.to_string();

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
                    if !line.trim_start().starts_with('#') {
                        line = line
                            .strip_prefix("    ")
                            .expect("a markdown indented code block must start with four spaces");

                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                }

                new_content = format!("```rust\n{new_content}```\n");

                out.replace_range(block.span.clone(), &new_content);
            } else {
                let mut new_content = String::new();

                for line in markdown[content.clone()].lines() {
                    if !line.trim_start().starts_with('#') {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                }

                out.replace_range(content, &new_content);
            }
        }

        if let CodeBlockKind::Fenced(_) = &block.kind {
            let line =
                out[block.span.start + 3..].lines().next().expect("it wouldn't be a code block ");
            let info_span = substr_range(&out, line);
            out.replace_range(info_span, "rust");
        }
    }

    out
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
        "ignore",
        "should_panic",
        "no_run",
        "compile_fail",
        "edition",
        "standalone_crate",
        "ignore",
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
    let mut out = markdown.to_string();

    for heading in headings(markdown).into_iter().rev() {
        out.insert(heading.span.start, '#');
    }

    out
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
