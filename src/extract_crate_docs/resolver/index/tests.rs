use std::{collections::HashMap, fmt, fs};

use expect_test::expect;
use rustdoc_types::{Crate, Id};

use crate::tests::TreeFormatter;

use super::{Tree, Value};

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
    let tree = Tree::new(&krate);

    expect![[r#"
        test_crate Module
        ├── MY_CONSTANT Constant
        ├── MY_STATIC Static
        ├── MyEnum Enum
        │   └── MyVariant Variant
        ├── MyExternType ExternType
        ├── MyStruct Struct
        │   ├── Error AssocType
        │   ├── Error AssocType
        │   ├── borrow Method
        │   ├── borrow_mut Method
        │   ├── from Method
        │   ├── into Method
        │   ├── my_field StructField
        │   ├── my_method Method
        │   ├── try_from Method
        │   ├── try_into Method
        │   └── type_id Method
        ├── MyStructAlias TypeAlias
        ├── MyTrait Trait
        │   ├── MY_ASSOCIATED_CONSTANT AssocConst
        │   └── MyAssociatedType AssocType
        ├── MyTraitAlias TraitAlias
        ├── MyUnion Union
        ├── ReexportInline Struct
        ├── ReexportPrivate Struct
        ├── my_function Function
        ├── my_macro Macro
        ├── my_module Module
        ├── reexport Module
        │   └── Reexport Struct
        ├── reexport_inline Module
        └── very Module
            └── nested Module
                └── module Module
    "#]]
    .assert_eq(&tree.to_string());
}

impl fmt::Display for Tree<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_tree(self))
    }
}

fn format_tree(tree: &Tree) -> String {
    let mut branches: HashMap<Id, Branch> = HashMap::new();

    for (&id, value) in &tree.inv_tree {
        branches.entry(id).or_insert(Branch { value, children: vec![] });

        if let Some(parent_id) = value.parent {
            let value = &tree.inv_tree[&parent_id];
            let branch = branches.entry(parent_id).or_insert(Branch { value, children: vec![] });
            branch.children.push(id);
        }
    }

    for branch in branches.values_mut() {
        branch.children.sort_by_key(|id| tree.inv_tree[id].name);
    }

    let mut roots = tree
        .inv_tree
        .iter()
        .filter_map(|(&i, v)| v.parent.is_none().then_some(i))
        .collect::<Vec<_>>();

    roots.sort_by_key(|id| tree.inv_tree[id].name);

    let mut out = String::new();

    for root in roots {
        out.push_str(&format_branch(&branches, root));
    }

    out
}

fn format_branch(branches: &HashMap<Id, Branch>, id: Id) -> String {
    let mut fmt = TreeFormatter::default();
    let Branch { value: Value { kind, name, .. }, children } = &branches[&id];

    let space = if name.is_empty() { "" } else { " " };
    fmt.label(format_args!("{name}{space}{kind:?}"));
    fmt.children(children.iter().map(|&id| format_branch(branches, id)));
    fmt.finish()
}

struct Branch<'a> {
    value: &'a Value<'a>,
    children: Vec<Id>,
}
