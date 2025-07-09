use expect_test::expect;

use super::{clean_code_blocks, code_blocks, find_section};

fn replace_section(markdown: &str, replacement: &str) -> String {
    let range = find_section(markdown, "section").unwrap();
    let mut out = markdown.to_string();
    out.replace_range(range, replacement);
    out
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
