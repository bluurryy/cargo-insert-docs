use expect_test::expect;

use crate::markdown::format_link_destination;

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

#[test]
fn test_format_link_destination() {
    assert_eq!(format_link_destination("foobar"), "foobar");
    assert_eq!(format_link_destination("foo<bar>baz"), "foo<bar>baz");

    assert_eq!(format_link_destination(""), "<>");
    assert_eq!(format_link_destination("<foo"), "<%3Cfoo>");
    assert_eq!(format_link_destination("foo bar"), "<foo bar>");
    assert_eq!(format_link_destination("foo()bar"), "<foo()bar>");
}
