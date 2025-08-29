use core::ops::Range;

use expect_test::expect;

use crate::{
    extract_crate_docs::rewrite_markdown::{
        RewriteMarkdownOptions, code_block_fence_is_rust, rewrite_markdown,
    },
    markdown_rs::{
        self,
        event::{Event, Kind},
    },
    tests::{TreeFormatterStack, events_to_string},
};

fn rewrite_markdown_default(markdown: &str) -> String {
    rewrite_markdown(markdown, &RewriteMarkdownOptions::default())
}

fn shrink_headings(markdown: &str, shrink_headings: i8) -> String {
    rewrite_markdown(markdown, &RewriteMarkdownOptions { shrink_headings, ..Default::default() })
}

#[test]
fn debug() {
    let source = r#"
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
"#;

    println!("{}", events_to_string(source));
    println!("{}", rewrite_markdown_default(source));
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
    .assert_eq(&rewrite_markdown_default(
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
fn test_code_block_fence_is_rust() {
    assert!(code_block_fence_is_rust(""));
    assert!(code_block_fence_is_rust("rust"));
    assert!(code_block_fence_is_rust("ignore"));
    assert!(code_block_fence_is_rust("should_panic"));
    assert!(code_block_fence_is_rust("no_run"));
    assert!(code_block_fence_is_rust("compile_fail"));
    assert!(code_block_fence_is_rust("edition"));
    assert!(code_block_fence_is_rust("standalone_crate"));
    assert!(code_block_fence_is_rust("ignore"));

    assert!(code_block_fence_is_rust("edition2015"));
    assert!(code_block_fence_is_rust("edition2018"));
    assert!(code_block_fence_is_rust("edition2021"));
    assert!(code_block_fence_is_rust("edition2024"));

    assert!(code_block_fence_is_rust("ignore-x86_64"));
    assert!(code_block_fence_is_rust("ignore-x86_64,ignore-windows"));

    assert!(!code_block_fence_is_rust("c"));
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
