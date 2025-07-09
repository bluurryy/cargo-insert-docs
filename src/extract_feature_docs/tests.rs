use expect_test::expect;
use indoc::indoc;

use super::{extract, parse};

#[test]
fn test_extract() {
    expect![[r#"
        - std *(enabled by default)* — Some docs about std
        - serde — Some docs about serde
        - something_undocumented
    "#]]
    .assert_eq(
        &extract(
            indoc! {r#"
                [features]
                default = ["std"]
                ## Some docs about std
                std = []
                ## Some docs about serde
                serde = []
                something_undocumented = []
            "#},
            "{feature}",
        )
        .unwrap(),
    );
}

#[test]
fn test_extract_inline() {
    expect![[r#"
        - std *(enabled by default)*
        - serde
    "#]]
    .assert_eq(
        &extract(
            indoc! {r#"
                features = { default = ["std"], std = [], serde = [] }
            "#},
            "{feature}",
        )
        .unwrap(),
    );
}

#[test]
fn test_feature_syntax_no_space() {
    expect!["a non-empty feature docs comment line must start with a space"]
        .assert_eq(&parse("[features]\n##Evil docs.\nmy_feature = []").unwrap_err().to_string());
}

#[test]
fn test_feature_syntax_no_space_in_empty_line() {
    assert_eq!(
        "\
- my_feature — Good
  \t \u{A0}
  docs.
",
        extract("[features]\n## Good\n##\t \u{A0}\n## docs.\nmy_feature = []", "{feature}")
            .unwrap(),
    );
}
