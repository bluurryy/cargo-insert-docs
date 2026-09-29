use std::{collections::HashMap, fmt};

use anstream::ColorChoice;
use cargo_metadata::{MetadataCommand, Package};
use expect_test::expect;
use rustdoc_types::Id;

use crate::{
    config::PackageConfigPatch,
    package_context::{CliContext, PackageContext},
    pretty_log::PrettyLog,
    rustdoc_json,
    tests::TreeFormatter,
};

use super::{Tree, Value};

fn get_package(path: &str) -> Package {
    MetadataCommand::new().manifest_path(path).exec().unwrap().workspace_default_packages()[0]
        .clone()
}

#[test]
fn test_tree() {
    let krate = rustdoc_json::generate(
        &PrettyLog::new(Box::new(anstream::AutoStream::new(std::io::stderr(), ColorChoice::Never))),
        &CliContext::default(),
        &PackageContext::resolve(
            &get_package(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test-crate/Cargo.toml")),
            &PackageConfigPatch { ..Default::default() }.finish(),
        )
        .unwrap()
        .expect("no target?"),
    )
    .unwrap();

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
        │   ├── MyAssociatedType AssocType
        │   ├── my_provided_method Method
        │   └── my_required_method TyMethod
        ├── MyTraitAlias TraitAlias
        ├── MyUnion Union
        ├── ReexportInline Struct
        ├── ReexportPrivate Struct
        ├── my_function Function
        ├── my_glob_imported_fn_from_private_mod Function
        ├── my_inline_glob_imported_fn Function
        ├── my_macro Macro
        ├── my_module Module
        ├── overloaded_name Function
        ├── overloaded_name Macro
        ├── overloaded_name Module
        │   └── something Function
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
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct SortKey<'a> {
        name: &'a str,
        kind: String,
    }

    let mut branches: HashMap<Id, Branch> = HashMap::new();

    for (&id, value) in &tree.inv_tree {
        branches.entry(id).or_insert(Branch { value, children: vec![] });

        if let Some(parent_id) = value.parent {
            let value = &tree.inv_tree[&parent_id];
            let branch = branches.entry(parent_id).or_insert(Branch { value, children: vec![] });
            branch.children.push(id);
        }
    }

    let get_sort_key = |id: &Id| {
        let item = &tree.inv_tree[id];
        SortKey { name: item.name, kind: format!("{:?}", item.kind) }
    };

    for branch in branches.values_mut() {
        branch.children.sort_by_key(get_sort_key);
    }

    let mut roots = tree
        .inv_tree
        .iter()
        .filter_map(|(&i, v)| v.parent.is_none().then_some(i))
        .collect::<Vec<_>>();

    roots.sort_by_key(get_sort_key);

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
