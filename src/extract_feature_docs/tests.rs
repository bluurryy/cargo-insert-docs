use color_eyre::eyre::Result;
use expect_test::expect;
use indoc::indoc;

use super::{comment_line_unprefixed, extract, parse};

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
        "- my_feature — Good\n  \n  docs.\n",
        extract("[features]\n## Good\n##\n## docs.\nmy_feature = []", "{feature}").unwrap(),
    );

    assert_eq!(
        "- my_feature — Good\n  \n  docs.\n",
        extract("[features]\n## Good\n##\t \u{A0}\n## docs.\nmy_feature = []", "{feature}")
            .unwrap(),
    );
}

#[test]
fn test_comment_line() {
    fn try_strip(s: &str) -> Result<&str> {
        comment_line_unprefixed(s)
    }

    #[track_caller]
    fn strip(s: &str) -> &str {
        try_strip(s).unwrap()
    }

    assert_eq!(strip(" "), "");
    assert_eq!(strip("\t"), "");
    assert_eq!(strip("\u{A0}"), "");

    assert_eq!(strip(" Hello"), "Hello");
    assert_eq!(strip("  Hello"), " Hello");

    assert!(try_strip("\tHello").is_err());
    assert!(try_strip("\u{A0}Hello").is_err());

    assert_eq!(strip(" Hello "), "Hello");
    assert_eq!(strip(" Hello  "), "Hello");
    assert_eq!(strip(" Hello\t"), "Hello");
    assert_eq!(strip(" Hello\u{A0}"), "Hello");
}
