#[cfg(test)]
mod tests;

use std::ops::Range;

use color_eyre::eyre::{Result, bail};
use rangemap::RangeMap;
use syn::spanned::Spanned as _;

use crate::markdown;

pub struct FeatureDocsSection<'a> {
    source: &'a str,
    docs: Docs,
    section: Range<usize>,
}

impl<'a> FeatureDocsSection<'a> {
    pub fn find(source: &'a str, section_name: &str) -> Result<Option<Self>> {
        let docs = parse(source)?;

        let Some(section) = markdown::find_section(&docs.value, section_name) else {
            return Ok(None);
        };

        Ok(Some(FeatureDocsSection { source, docs, section }))
    }

    // TODO: format comments like cargo fmt would, removing whitespace on empty lines (?)
    pub fn replace(&self, section_content: &str) -> Result<String> {
        let Self { source, docs, section } = self;

        let start = section.start;
        let end = section.end;

        let start_frag_i = *docs.source_map.get(&start).unwrap();
        let end_frag_i = *docs.source_map.get(&end).unwrap();

        let start_frag = &docs.frags[start_frag_i];
        let end_frag = &docs.frags[end_frag_i];

        if start_frag_i == end_frag_i {
            bail!("section start and end in the same doc attribute is not yet supported");
        }

        // Ideally we'd remove the text before the end marker within the same attribute
        // and then the text after the start marker within the same attribute.
        //
        // But this is very hard. We'd have to keep track of the source map in the beautify
        // and unindent functions and we'd need to parse the string literal ourselves like
        // syn does but also keeping track of a source map.
        //
        // Sure we could just half-ass it and search for the marker string using `find` but that
        // would go against the effort of parsing things properly.

        let replacement = {
            let mut out = String::new();
            out.push('\n');

            for line in section_content.lines() {
                out.push_str("//!");

                // rustfmt makes whitespace lines empty, so we do too
                if !line.chars().all(char::is_whitespace) {
                    out.push(' ');
                    out.push_str(line);
                }

                out.push('\n');
            }

            out
        };

        let mut out = source.to_string();

        let insert_start = start_frag.attr_span.end;
        let insert_end = end_frag.attr_span.start;

        out.replace_range(insert_start..insert_end, &replacement);

        // after the attribute end there was probably already a newline
        // so no need for a second one
        let after_insertion = insert_start + replacement.len();
        if out[after_insertion..].starts_with('\n') {
            out.remove(after_insertion);
        }

        Ok(out)
    }
}

fn parse(lib_rs: &str) -> Result<Docs> {
    let fragments = parse_doc_frags(lib_rs)?;
    Ok(combine_doc_frags(fragments))
}

#[derive(Clone, Debug)]
pub struct DocFragment {
    attr_span: Range<usize>,
    #[expect(dead_code)]
    lit_span: Range<usize>,
    doc: String,
    kind: DocFragmentKind,
    #[expect(dead_code)]
    comment_kind: CommentKind,
    indent: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DocFragmentKind {
    SugaredDoc,
    RawDoc,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum CommentKind {
    Block,
    Line,
}

#[derive(Debug, Default)]
pub struct Docs {
    value: String,
    source_map: SourceMap,
    frags: Vec<DocFragment>,
}

fn parse_doc_frags(lib_rs: &str) -> Result<Vec<DocFragment>> {
    let file = syn::parse_file(lib_rs)?;

    let mut doc_fragments = vec![];

    for attr in &file.attrs {
        if !matches!(attr.style, syn::AttrStyle::Inner { .. }) {
            continue;
        }

        let syn::MetaNameValue { path, value, .. } = match &attr.meta {
            syn::Meta::NameValue(meta_name_value) => meta_name_value,
            _ => continue,
        };

        if !path.is_ident("doc") {
            continue;
        }

        let syn::Expr::Lit(lit) = value else { continue };

        let syn::Lit::Str(lit_str) = &lit.lit else {
            continue;
        };

        let raw_attr = &lib_rs[attr.span().byte_range()];

        let fragment_kind;
        let comment_kind;

        match raw_attr.get(..3).unwrap_or("") {
            "//!" => {
                fragment_kind = DocFragmentKind::SugaredDoc;
                comment_kind = CommentKind::Line;
            }
            "/*!" => {
                fragment_kind = DocFragmentKind::SugaredDoc;
                comment_kind = CommentKind::Block;
            }
            "#![" => {
                fragment_kind = DocFragmentKind::RawDoc;
                comment_kind = CommentKind::Line;
            }
            _ => {
                let i = raw_attr.char_indices().take(3).map(|(i, _)| i).last().unwrap_or(0);
                let starts_with = &raw_attr[..i];
                bail!(
                    "doc attribute starts with {starts_with:?}, expected either \"//!\", \"/*!\" or \"#![\""
                )
            }
        }

        doc_fragments.push(DocFragment {
            attr_span: attr.span().byte_range(),
            lit_span: lit_str.span().byte_range(),
            doc: beautify_doc_string(lit_str.value(), comment_kind),
            kind: fragment_kind,
            comment_kind,
            indent: 0,
        });
    }

    unindent_doc_fragments(&mut doc_fragments);

    Ok(doc_fragments)
}

type SourceMap = RangeMap<usize, usize>;

fn combine_doc_frags(frags: Vec<DocFragment>) -> Docs {
    #[derive(Default)]
    struct DocsBuilder {
        value: String,
        source_map: SourceMap,
    }

    impl DocsBuilder {
        fn push(&mut self, i: usize, value: &str) {
            let start = self.value.len();
            self.value.push_str(value);
            self.value.push('\n');
            let end = self.value.len();

            // the newline is not part of the source
            // but it does not matter for our purposes
            self.source_map.insert(start..end, i);
        }

        fn build(self, frags: Vec<DocFragment>) -> Docs {
            Docs { value: self.value, source_map: self.source_map, frags }
        }
    }

    let mut docs = DocsBuilder::default();

    for (i, frag) in frags.iter().enumerate() {
        for line in frag.doc.lines() {
            if !line.chars().all(char::is_whitespace) {
                docs.push(i, &line[frag.indent..]);
            } else {
                docs.push(i, line);
            }
        }
    }

    docs.build(frags)
}

// From rustlang/compiler/rustc_resolve/src/rustdoc.rs
//
/// Removes excess indentation on comments in order for the Markdown
/// to be parsed correctly. This is necessary because the convention for
/// writing documentation is to provide a space between the /// or //! marker
/// and the doc text, but Markdown is whitespace-sensitive. For example,
/// a block of text with four-space indentation is parsed as a code block,
/// so if we didn't unindent comments, these list items
///
/// /// A list:
/// ///
/// ///    - Foo
/// ///    - Bar
///
/// would be parsed as if they were in a code block, which is likely not what the user intended.
fn unindent_doc_fragments(docs: &mut [DocFragment]) {
    // `add` is used in case the most common sugared doc syntax is used ("/// "). The other
    // fragments kind's lines are never starting with a whitespace unless they are using some
    // markdown formatting requiring it. Therefore, if the doc block have a mix between the two,
    // we need to take into account the fact that the minimum indent minus one (to take this
    // whitespace into account).
    //
    // For example:
    //
    // /// hello!
    // #[doc = "another"]
    //
    // In this case, you want "hello! another" and not "hello!  another".
    let add = if docs.windows(2).any(|arr| arr[0].kind != arr[1].kind)
        && docs.iter().any(|d| d.kind == DocFragmentKind::SugaredDoc)
    {
        // In case we have a mix of sugared doc comments and "raw" ones, we want the sugared one to
        // "decide" how much the minimum indent will be.
        1
    } else {
        0
    };

    // `min_indent` is used to know how much whitespaces from the start of each lines must be
    // removed. Example:
    //
    // ///     hello!
    // #[doc = "another"]
    //
    // In here, the `min_indent` is 1 (because non-sugared fragment are always counted with minimum
    // 1 whitespace), meaning that "hello!" will be considered a codeblock because it starts with 4
    // (5 - 1) whitespaces.
    let Some(min_indent) = docs
        .iter()
        .map(|fragment| {
            fragment
                .doc
                .as_str()
                .lines()
                .filter(|line| line.chars().any(|c| !c.is_whitespace()))
                .map(|line| {
                    // Compare against either space or tab, ignoring whether they are
                    // mixed or not.
                    let whitespace = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                    whitespace
                        + (if fragment.kind == DocFragmentKind::SugaredDoc { 0 } else { add })
                })
                .min()
                .unwrap_or(usize::MAX)
        })
        .min()
    else {
        return;
    };

    for fragment in docs {
        let indent = if fragment.kind != DocFragmentKind::SugaredDoc && min_indent > 0 {
            min_indent - add
        } else {
            min_indent
        };

        fragment.indent = indent;
    }
}

// From rustlang/compiler/rustc_ast/src/util/comments.rs
//
/// Makes a doc string more presentable to users.
/// Used by rustdoc and perhaps other tools, but not by rustc.
fn beautify_doc_string(data: String, kind: CommentKind) -> String {
    fn get_vertical_trim(lines: &[&str]) -> Option<(usize, usize)> {
        let mut i = 0;
        let mut j = lines.len();
        // first line of all-stars should be omitted
        if lines.first().is_some_and(|line| line.chars().all(|c| c == '*')) {
            i += 1;
        }

        // like the first, a last line of all stars should be omitted
        if j > i && !lines[j - 1].is_empty() && lines[j - 1].chars().all(|c| c == '*') {
            j -= 1;
        }

        if i != 0 || j != lines.len() { Some((i, j)) } else { None }
    }

    fn get_horizontal_trim(lines: &[&str], kind: CommentKind) -> Option<String> {
        let mut i = usize::MAX;
        let mut first = true;

        // In case we have doc comments like `/**` or `/*!`, we want to remove stars if they are
        // present. However, we first need to strip the empty lines so they don't get in the middle
        // when we try to compute the "horizontal trim".
        let lines = match kind {
            CommentKind::Block => {
                // Whatever happens, we skip the first line.
                let mut i = lines
                    .first()
                    .map(|l| if l.trim_start().starts_with('*') { 0 } else { 1 })
                    .unwrap_or(0);
                let mut j = lines.len();

                while i < j && lines[i].trim().is_empty() {
                    i += 1;
                }
                while j > i && lines[j - 1].trim().is_empty() {
                    j -= 1;
                }
                &lines[i..j]
            }
            CommentKind::Line => lines,
        };

        for line in lines {
            for (j, c) in line.chars().enumerate() {
                if j > i || !"* \t".contains(c) {
                    return None;
                }
                if c == '*' {
                    if first {
                        i = j;
                        first = false;
                    } else if i != j {
                        return None;
                    }
                    break;
                }
            }
            if i >= line.len() {
                return None;
            }
        }
        Some(lines.first()?[..i].to_string())
    }

    let data_s = data.as_str();
    if data_s.contains('\n') {
        let mut lines = data_s.lines().collect::<Vec<&str>>();
        let mut changes = false;
        let lines = if let Some((i, j)) = get_vertical_trim(&lines) {
            changes = true;
            // remove whitespace-only lines from the start/end of lines
            &mut lines[i..j]
        } else {
            &mut lines
        };
        if let Some(horizontal) = get_horizontal_trim(lines, kind) {
            changes = true;
            // remove a "[ \t]*\*" block from each line, if possible
            for line in lines.iter_mut() {
                if let Some(tmp) = line.strip_prefix(&horizontal) {
                    *line = tmp;
                    if kind == CommentKind::Block
                        && (*line == "*" || line.starts_with("* ") || line.starts_with("**"))
                    {
                        *line = &line[1..];
                    }
                }
            }
        }
        if changes {
            return lines.join("\n");
        }
    }

    data
}
