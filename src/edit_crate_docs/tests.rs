use color_eyre::eyre::Result;
use expect_test::expect;
use indoc::indoc;

use super::FeatureDocsSection;

fn replace_section(
    source: &str,
    section_name: &str,
    section_content: &str,
) -> Result<Option<String>> {
    if let Some(section) = FeatureDocsSection::find(source, section_name)? {
        section.replace(section_content).map(Some)
    } else {
        Ok(None)
    }
}

#[test]
fn raw() {
    expect![[r##"
        #![doc = "prefix"]
        #![doc = "keep <!-- section start --> remove"]
        //! multi
        //! line
        //! content
        #![doc = "remove <!-- section end --> keep"]
        #![doc = "suffix"]
    "##]]
    .assert_eq(
        &replace_section(
            indoc! {r#"
            #![doc = "prefix"]
            #![doc = "keep <!-- section start --> remove"]
            #![doc = "remove <!-- section end --> keep"]
            #![doc = "suffix"]
            "#},
            "section",
            "multi\nline\ncontent",
        )
        .unwrap()
        .unwrap(),
    );
}

#[test]
fn line() {
    expect![[r#"
        //! prefix
        //! keep <!-- section start --> remove
        //! multi
        //! line
        //! content
        //! remove <!-- section end --> keep
        //! suffix
    "#]]
    .assert_eq(
        &replace_section(
            indoc! {r#"
            //! prefix
            //! keep <!-- section start --> remove
            //! remove <!-- section end --> keep
            //! suffix
            "#},
            "section",
            "multi\nline\ncontent",
        )
        .unwrap()
        .unwrap(),
    );
}

#[test]
#[ignore = "todo"]
fn block() {
    expect![[r#""#]].assert_eq(
        &replace_section(
            indoc! {r#"
            /*! prefix
             * keep <!-- section start --> remove
             * remove <!-- section end --> keep
             * suffix
             */
            "#},
            "section",
            "multi\nline\ncontent",
        )
        .unwrap()
        .unwrap(),
    );
}

#[test]
fn block_separate() {
    expect![[r#"
        /*! prefix
         * keep <!-- section start --> remove
         */
        //! multi
        //! line
        //! content
        /*! remove <!-- section end --> keep
         * suffix
         */
    "#]]
    .assert_eq(
        &replace_section(
            indoc! {r#"
            /*! prefix
             * keep <!-- section start --> remove
             */
            /*! remove <!-- section end --> keep
             * suffix
             */
            "#},
            "section",
            "multi\nline\ncontent",
        )
        .unwrap()
        .unwrap(),
    );
}

#[test]
fn test_escaped_section() {
    let lib_rs = indoc! {r#"
        //! ```text
        //! <!-- feature documentation start -->
        //! <!-- feature documentation end -->
        //! ```
    "#};

    let new_lib_rs = replace_section(lib_rs, "feature documentation", "whatever").unwrap();

    assert!(new_lib_rs.is_none());
}

#[test]
fn test_trim_end() {
    let lib_rs = indoc! {r#"
        //! <!-- feature documentation start -->
        //! <!-- feature documentation end -->
    "#};

    let new_lib_rs = replace_section(
        lib_rs,
        "feature documentation",
        "lots of whitespace for some reason:    \t   \u{A0}",
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        "\
        //! <!-- feature documentation start -->\n\
        //! lots of whitespace for some reason:\n\
        //! <!-- feature documentation end -->\n",
        new_lib_rs
    );
}
