use core::ops::Range;

use expect_test::expect;

use crate::{
    markdown::{parse, parse_options},
    markdown_rs::event::{Event, Kind},
    tests::TreeFormatterStack,
};

use super::{find_section, find_subsections};

fn replace_section(markdown: &str, replacement: &str) -> String {
    let section = find_section(markdown, "section").unwrap();
    let mut out = markdown.to_string();
    out.replace_range(section.content_span, replacement);
    out
}

#[test]
fn test_find_section() {
    let markdown = r#"
before section
<!-- my section start -->
inside section
<!-- my section end -->
after section
    "#;

    let section = find_section(markdown, "my section").unwrap();

    expect![[r#"
        (
            "<!-- my section start -->\ninside section\n<!-- my section end -->",
            "\ninside section\n",
        )
    "#]]
    .assert_debug_eq(&(&markdown[section.span], &markdown[section.content_span]));
}

#[test]
fn test_find_subsections() {
    let markdown = r#"
before sections
<!-- my section start -->
will be ignored
<!-- my section end -->
<!-- my section foo start -->
foo
<!-- my section foo end -->
<!-- my section ignore this -->
<!-- my section bar start -->
bar
<!-- my section bar end -->
after sections
    "#;

    let result = find_subsections(markdown, "my section")
        .unwrap()
        .into_iter()
        .rev()
        .map(|(range, name)| (name, &markdown[range.span], &markdown[range.content_span]))
        .collect::<Vec<_>>();

    expect![[r#"
        [
            (
                "foo",
                "<!-- my section foo start -->\nfoo\n<!-- my section foo end -->",
                "\nfoo\n",
            ),
            (
                "bar",
                "<!-- my section bar start -->\nbar\n<!-- my section bar end -->",
                "\nbar\n",
            ),
        ]
    "#]]
    .assert_debug_eq(&result);
}

#[test]
fn test_find_subsections_multiple_in_flow() {
    // this doesn't really make sense for our use case, but
    // its better to just handle these as subsections
    // rather than ignoring them
    let markdown = r#"
<div>
<!-- my section foo start -->
<!-- my section foo end -->
<!-- my section bar start -->
<!-- my section bar end -->
</div>
"#;

    let result = find_subsections(markdown, "my section")
        .unwrap()
        .into_iter()
        .rev()
        .map(|(range, name)| (name, &markdown[range.span], &markdown[range.content_span]))
        .collect::<Vec<_>>();

    expect![[r#"
        [
            (
                "foo",
                "<!-- my section foo start -->\n<!-- my section foo end -->",
                "\n",
            ),
            (
                "bar",
                "<!-- my section bar start -->\n<!-- my section bar end -->",
                "\n",
            ),
        ]
    "#]]
    .assert_debug_eq(&result);

    let foo = find_section(markdown, "my section foo").unwrap();
    expect![[r#"
        (
            "foo",
            "<!-- my section foo start -->\n<!-- my section foo end -->",
            "\n",
        )
    "#]]
    .assert_debug_eq(&("foo", &markdown[foo.span], &markdown[foo.content_span]));

    let bar = find_section(markdown, "my section bar").unwrap();
    expect![[r#"
        (
            "bar",
            "<!-- my section bar start -->\n<!-- my section bar end -->",
            "\n",
        )
    "#]]
    .assert_debug_eq(&("bar", &markdown[bar.span], &markdown[bar.content_span]));
}

#[test]
fn test_replace_section_html() {
    expect![[r#"

        prefix
        <!-- section start -->
        NEW CONTENT
        <!-- section end -->
        suffix
    "#]]
    .assert_eq(&replace_section(
        r#"
prefix
<!-- section start -->
old content
<!-- section end -->
suffix
"#,
        "\nNEW CONTENT\n",
    ));
}

#[test]
fn test_replace_section_inline_html() {
    expect!["prefix <!-- section start -->NEW CONTENT<!-- section end --> suffix"].assert_eq(
        &replace_section(
            "prefix <!-- section start --> old content <!-- section end --> suffix",
            "NEW CONTENT",
        ),
    );
}

#[allow(dead_code)]
pub fn print_events(markdown: &str) {
    println!("{}", events_to_string(markdown))
}

#[allow(dead_code)]
pub fn events_to_string(markdown: &str) -> String {
    fn events_to_string(events: &[Event], source: &str) -> String {
        let mut fmt = TreeFormatterStack::new();

        fmt.push();
        fmt.label("Document");
        fmt.child_len(children(events));

        for (i, event) in events.iter().enumerate() {
            match event.kind {
                Kind::Enter => {
                    fmt.push();
                    let name = &event.name;
                    let range = range(&events[i..]);
                    let text = &source[range];
                    let link = event
                        .link
                        .as_ref()
                        .map(|link| format!(" {{ link: {link:?} }}"))
                        .unwrap_or_default();
                    fmt.label(format!("{name:?} {text:?}{link}"));
                    fmt.child_len(children(&events[i + 1..]));
                }
                Kind::Exit => fmt.pop(),
            }
        }

        fmt.finish()
    }

    fn range(events: &[Event]) -> Range<usize> {
        let start = events[0].point.index;
        let mut depth = 0usize;

        for event in &events[1..] {
            match event.kind {
                Kind::Enter => depth += 1,
                Kind::Exit => match depth.checked_sub(1) {
                    Some(new_depth) => depth = new_depth,
                    None => return start..event.point.index,
                },
            }
        }

        start..start
    }

    fn children(events: &[Event]) -> usize {
        let mut depth = 0usize;
        let mut count = 0usize;

        for event in events {
            match event.kind {
                Kind::Enter => {
                    if depth == 0 {
                        count += 1;
                    }

                    depth += 1;
                }
                Kind::Exit => match depth.checked_sub(1) {
                    Some(new_depth) => depth = new_depth,
                    None => return count,
                },
            }
        }

        count
    }

    let (events, _state) = parse(markdown, &parse_options());

    events_to_string(&events, markdown)
}
