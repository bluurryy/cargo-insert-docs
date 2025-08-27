use std::{collections::HashMap, fmt, fs};

use cargo_metadata::MetadataCommand;
use expect_test::expect;
use rustdoc_types::{Crate, Id};

use crate::{rustdoc_json, tests::TreeFormatter};

use super::{Tree, Value};

#[test]
fn test_tree() {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

    let metadata =
        &MetadataCommand::new().manifest_path(format!("{MANIFEST_DIR}/Cargo.toml")).exec().unwrap();

    let package = metadata.packages.iter().find(|p| p.name.as_str() == "test-crate").unwrap();
    let package_target = package.targets.iter().find(|t| t.is_lib()).unwrap();

    let (_, path) = rustdoc_json::generate(rustdoc_json::Options {
        metadata,
        package,
        package_target,
        toolchain: Some("nightly"),
        all_features: false,
        no_default_features: false,
        features: &mut None.into_iter(),
        manifest_path: None,
        target: None,
        target_dir: None,
        quiet: false,
        document_private_items: false,
        no_deps: false,
        output: rustdoc_json::CommandOutput::Inherit,
    })
    .unwrap();

    let json = fs::read_to_string(path).expect("failed to read generated rustdoc json");
    let krate: Crate = serde_json::from_str(&json).expect("failed to parse generated rustdoc json");
    let tree = Tree::new(&krate).unwrap();

    expect![[r#"
        test_crate Module
        ├── MY_CONSTANT Constant
        ├── MY_STATIC Static
        ├── MyEnum Enum
        │   └── MyVariant Variant
        ├── MyExternType ExternType
        ├── MyGlobImportedStructFromPrivateMod Struct
        ├── MyInlineGlobImportedStruct Struct
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
        ├── my_glob_imported_fn_from_private_mod Function
        ├── my_inline_glob_imported_fn Function
        ├── my_macro Macro
        ├── my_module Module
        ├── reexport Module
        │   └── Reexport Struct
        ├── reexport_inline Module
        ├── to_be_glob_imported Module
        │   ├── MyGlobImportedStruct Struct
        │   └── my_glob_imported_fn Function
        ├── to_be_inline_glob_imported Module
        └── very Module
            └── nested Module
                └── module Module
        to_be_glob_imported_private Module
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
