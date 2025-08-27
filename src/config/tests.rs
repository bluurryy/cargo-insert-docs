use serde::{Deserialize, Serialize};

use crate::config::{BoolOrString, TargetSelection, serialize_target_selection};

#[test]
fn test_target_selection() {
    #[derive(Debug, Default, Serialize, PartialEq, Eq)]
    #[serde(default)]
    struct Table {
        #[serde(flatten, serialize_with = "serialize_target_selection")]
        foo: Option<TargetSelection>,
    }

    assert_eq!(toml::to_string(&Table { foo: None }).unwrap(), "");
    assert_eq!(
        toml::to_string(&Table { foo: Some(TargetSelection::Lib) }).unwrap(),
        "lib = true\n"
    );
    assert_eq!(
        toml::to_string(&Table { foo: Some(TargetSelection::Bin(None)) }).unwrap(),
        "bin = true\n"
    );
    assert_eq!(
        toml::to_string(&Table { foo: Some(TargetSelection::Bin(Some("hey".into()))) }).unwrap(),
        "bin = \"hey\"\n"
    );
}

#[test]
fn test_bool_or_string() {
    #[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(default)]
    struct Table {
        foo: Option<BoolOrString>,
    }

    assert_eq!(toml::to_string(&Table { foo: None }).unwrap(), "");
    assert_eq!(
        toml::to_string(&Table { foo: Some(BoolOrString::Bool(false)) }).unwrap(),
        "foo = false\n"
    );
    assert_eq!(
        toml::to_string(&Table { foo: Some(BoolOrString::Bool(true)) }).unwrap(),
        "foo = true\n"
    );
    assert_eq!(
        toml::to_string(&Table { foo: Some(BoolOrString::String("hey".into())) }).unwrap(),
        "foo = \"hey\"\n"
    );

    assert_eq!(toml::from_str(""), Ok(Table { foo: None }));
    assert_eq!(toml::from_str("foo = true"), Ok(Table { foo: Some(BoolOrString::Bool(true)) }));
    assert_eq!(toml::from_str("foo = false"), Ok(Table { foo: Some(BoolOrString::Bool(false)) }));
    assert_eq!(
        toml::from_str("foo = 'bar'"),
        Ok(Table { foo: Some(BoolOrString::String(String::from("bar"))) })
    );
}
