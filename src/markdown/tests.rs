use expect_test::expect;

use crate::markdown::shrink_headings;

use super::{
    clean_code_blocks, code_blocks, fenced_code_block_is_rust, find_section, find_subsections,
};

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

#[test]
fn test_clean_code_blocks() {
    expect![[r#"

        ```rust
        // this is rust code
        let one = 1;
        let two = 2;
        assert_eq!(one + two, 3);
        ```

        ```rust
        // this is rust code too
        let one = 1;
        let two = 2;
        assert_eq!(one + two, 3);
        ```

        ```rust
        // this is also rust code believe it or not
        let one = 1;
        let two = 2;
        assert_eq!(one + two, 3);
        ```

        ```python
        # this most certainly isn't though
        def square(n):
            n * n
        ```
    "#]]
    .assert_eq(&clean_code_blocks(
        r#"
```
// this is rust code
let one = 1;
# println!("won't show up in readme");
let two = 2;
assert_eq!(one + two, 3);
```

```compile_fail,E69420
// this is rust code too
let one = 1;
# println!("won't show up in readme");
let two = 2;
assert_eq!(one + two, 3);
```

    // this is also rust code believe it or not
    let one = 1;
    # println!("won't show up in readme");
    let two = 2;
    assert_eq!(one + two, 3);

```python
# this most certainly isn't though
def square(n):
    n * n
```
"#,
    ));
}

#[test]
fn test_indented_code_blocks() {
    assert_eq!(code_blocks("    block")[0].span.start, 0);
    assert_eq!(code_blocks("\n    block")[0].span.start, 1);
}

#[test]
fn test_fenced_code_block_is_rust() {
    assert!(fenced_code_block_is_rust(""));
    assert!(fenced_code_block_is_rust("rust"));
    assert!(fenced_code_block_is_rust("ignore"));
    assert!(fenced_code_block_is_rust("should_panic"));
    assert!(fenced_code_block_is_rust("no_run"));
    assert!(fenced_code_block_is_rust("compile_fail"));
    assert!(fenced_code_block_is_rust("edition"));
    assert!(fenced_code_block_is_rust("standalone_crate"));
    assert!(fenced_code_block_is_rust("ignore"));

    assert!(fenced_code_block_is_rust("edition2015"));
    assert!(fenced_code_block_is_rust("edition2018"));
    assert!(fenced_code_block_is_rust("edition2021"));
    assert!(fenced_code_block_is_rust("edition2024"));

    assert!(fenced_code_block_is_rust("ignore-x86_64"));
    assert!(fenced_code_block_is_rust("ignore-x86_64,ignore-windows"));

    assert!(!fenced_code_block_is_rust("c"));
}

#[test]
fn test_shrink_headings() {
    assert_eq!(shrink_headings("## foo", -3), "# foo");
    assert_eq!(shrink_headings("## foo", -2), "# foo");
    assert_eq!(shrink_headings("## foo", -1), "# foo");
    assert_eq!(shrink_headings("## foo", 0), "## foo");
    assert_eq!(shrink_headings("## foo", 1), "### foo");
    assert_eq!(shrink_headings("## foo", 2), "#### foo");
    assert_eq!(shrink_headings("## foo", 3), "##### foo");
    assert_eq!(shrink_headings("## foo", 4), "###### foo");
    assert_eq!(shrink_headings("## foo", 5), "###### foo");
    assert_eq!(shrink_headings("## foo", 6), "###### foo");

    assert_eq!(shrink_headings("  ####   foo", -2), "  ##   foo");
}
