use std::{collections::HashMap, fmt};

use expect_test::expect;
use rustdoc_types::Id;

use crate::tests::TreeFormatter;

use super::{Tree, Value};

macro_rules! paths {
    ($(
        $id:literal: $kind:ident {
            $($path:ident)*
        }
    )*) => {
        super::Paths::from_iter(vec![
            $((
                rustdoc_types::Id($id),
                rustdoc_types::ItemSummary {
                    crate_id: 0,
                    path: vec![$(stringify!($path).to_string()),*],
                    kind: rustdoc_types::ItemKind::$kind,
                }
            ),)*
        ])
    };
}

#[test]
fn test_simple() {
    let paths = paths! {
        0: Function { std io Write write }
        1: Trait { std io Write }
        2: Module { std io }
        2: Module { std }
    };

    let tree = Tree::new_simple(&paths);
    let path = tree.path_to(Id(0)).unwrap();

    expect![[r#"
        [
            PathItem {
                name: "write",
                kind: Function,
            },
            PathItem {
                name: "Write",
                kind: Trait,
            },
            PathItem {
                name: "io",
                kind: Module,
            },
            PathItem {
                name: "std",
                kind: Module,
            },
        ]
    "#]]
    .assert_debug_eq(&path);
}

#[test]
fn test_partially_dangling() {
    let paths = paths! {
        0: Trait { std io Write }
    };

    let tree = Tree::new_simple(&paths);
    let path = tree.path_to(Id(0)).unwrap();

    expect![[r#"
        [
            PathItem {
                name: "Write",
                kind: Trait,
            },
            PathItem {
                name: "io",
                kind: Module,
            },
            PathItem {
                name: "std",
                kind: Module,
            },
        ]
    "#]]
    .assert_debug_eq(&path);
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
