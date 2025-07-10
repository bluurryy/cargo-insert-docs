use std::{collections::BTreeMap, fs};

use expect_test::expect;
use rustdoc_types::{Crate, Id};

use super::{super::BasicItemKind, Parent, child_to_parent};

#[test]
fn test_item_paths() {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

    let json_path = rustdoc_json::Builder::default()
        .toolchain("nightly")
        .manifest_path(format!("{MANIFEST_DIR}/tests/test-crate/Cargo.toml"))
        .all_features(true)
        .build()
        .unwrap();

    let json = fs::read_to_string(json_path).expect("failed to read generated rustdoc json");
    let krate: Crate = serde_json::from_str(&json).expect("failed to parse generated rustdoc json");
    let child_to_parent = child_to_parent(&krate).unwrap();
    let tree = format_child_to_parent(&krate, &child_to_parent);

    // TODO: collapse impl
    expect![[r#"
        test_crate Module
        ├── MyExternType ExternType
        ├── reexport Module
        │   └── Reexport Struct
        ├── very Module
        │   └── nested Module
        │       └── module Module
        ├── ReexportInline Struct
        ├── reexport_inline Module
        ├── ReexportPrivate Struct
        ├── my_module Module
        ├── alloc ExternCrate
        ├── MyStruct Struct
        │   ├── my_field StructField
        │   ├── _ Impl
        │   │   └── my_method Function
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   │   └── borrow Function
        │   ├── _ Impl
        │   │   └── borrow_mut Function
        │   ├── _ Impl
        │   │   └── into Function
        │   ├── _ Impl
        │   │   └── from Function
        │   ├── _ Impl
        │   │   ├── Error AssocType
        │   │   └── try_into Function
        │   ├── _ Impl
        │   │   ├── Error AssocType
        │   │   └── try_from Function
        │   └── _ Impl
        │       └── type_id Function
        ├── MyUnion Union
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   └── _ Impl
        ├── MyEnum Enum
        │   ├── MyVariant Variant
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   ├── _ Impl
        │   └── _ Impl
        ├── my_function Function
        ├── MyTrait Trait
        │   ├── MY_ASSOCIATED_CONSTANT AssocConst
        │   └── MyAssociatedType AssocType
        ├── MyTraitAlias TraitAlias
        ├── MyStructAlias TypeAlias
        ├── MY_CONSTANT Constant
        ├── MY_STATIC Static
        └── my_macro Macro
    "#]]
    .assert_eq(&tree);
}

fn format_child_to_parent(krate: &Crate, child_to_parent: &BTreeMap<Id, Id>) -> String {
    let mut parent_to_child = BTreeMap::<Id, Vec<Id>>::new();

    for (&child, &parent) in child_to_parent {
        parent_to_child.entry(parent).or_default().push(child);
    }

    format_parent_to_child(krate, &parent_to_child, krate.root)
}

#[expect(dead_code)]
fn format_child_to_parent_struct(krate: &Crate, child_to_parent: &BTreeMap<Id, Parent>) -> String {
    let mut parent_to_child = BTreeMap::<Id, Vec<Id>>::new();

    for (&child, parent) in child_to_parent {
        parent_to_child.entry(parent.id).or_default().push(child);
    }

    format_parent_to_child(krate, &parent_to_child, krate.root)
}

fn format_parent_to_child(
    krate: &Crate,
    parent_to_child: &BTreeMap<Id, Vec<Id>>,
    id: Id,
) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    let item = &krate.index[&id];

    if let Some(name) = &item.name {
        write!(out, "{name}").unwrap();
    } else {
        write!(out, "_").unwrap();
    };

    let kind = BasicItemKind::from(item);

    writeln!(out, " {kind:?}").unwrap();

    let Some(children) = parent_to_child.get(&id) else {
        return out;
    };

    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len().wrapping_sub(1);

        for (i, line) in format_parent_to_child(krate, parent_to_child, *child).lines().enumerate()
        {
            #[expect(clippy::collapsible_else_if)]
            let indent = if i == 0 {
                if is_last { "└── " } else { "├── " }
            } else {
                if is_last { "    " } else { "│   " }
            };

            out.push_str(indent);
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}
