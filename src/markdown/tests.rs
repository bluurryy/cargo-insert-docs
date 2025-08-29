use expect_test::expect;

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
