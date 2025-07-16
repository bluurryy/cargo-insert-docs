//! Processes `.paths`.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rustdoc_types::{Crate, Id, ItemKind, ItemSummary};

use crate::extract_crate_docs::resolver::{Kind, PathItem};

type Paths = HashMap<Id, ItemSummary>;

pub struct Tree<'a> {
    paths: &'a Paths,
    inv_tree: HashMap<Id, Value<'a>>,
}

impl<'a> Tree<'a> {
    pub fn new(krate: &'a Crate) -> Self {
        Self::new_simple(&krate.paths)
    }

    fn new_simple(paths: &'a Paths) -> Self {
        let parents = parents(paths);
        let mut inv_tree = HashMap::new();

        for child_id in paths.keys().copied() {
            let child_item = &paths[&child_id];
            let child_name = item_name(child_item);

            let Some(mut child_kind) = item_kind(child_item) else {
                continue;
            };

            let parent_id = {
                if let Some(&(mut parent_id)) = parents.get(&child_id) {
                    let parent_item = &paths[&parent_id];

                    // Afaict this won't happen currently, paths to methods or impls
                    // won't even get generated. But since the `ItemKind` exists, it
                    // makes sense for us to handle this here. Perhaps in the future
                    // this will do something.
                    if matches!(parent_item.kind, ItemKind::Impl)
                        && let Some(&grand_parent_id) = parents.get(&parent_id)
                    {
                        parent_id = grand_parent_id;
                        child_kind = Kind::Method;
                    }

                    Some(parent_id)
                } else {
                    None
                }
            };

            inv_tree
                .insert(child_id, Value { parent: parent_id, kind: child_kind, name: child_name });
        }

        Self { paths, inv_tree }
    }

    pub fn path_to(&self, mut id: Id) -> Option<Vec<PathItem<'a>>> {
        let mut path = vec![];

        while let Some(&Value { parent, kind, name }) = self.inv_tree.get(&id) {
            path.push(PathItem { name, kind });
            let Some(parent) = parent else { break };
            id = parent;
        }

        if path.is_empty() {
            return None;
        }

        // `.paths` may not contain entries for all ancestors.
        // We assume the remaining ancestors are modules.
        if let Some(remaining_path) = without_last(&self.paths[&id].path) {
            for name in remaining_path.iter().rev() {
                path.push(PathItem { name, kind: Kind::Module });
            }
        }

        Some(path)
    }
}

fn without_last<T>(slice: &[T]) -> Option<&[T]> {
    let end = slice.len().checked_sub(1)?;
    slice.get(..end)
}

#[derive(Debug, Clone, Copy)]
struct Value<'a> {
    parent: Option<Id>,
    kind: Kind,
    name: &'a str,
}

fn parents(paths: &Paths) -> HashMap<Id, Id> {
    let path_to_id = path_to_id(paths);
    let mut parents = HashMap::new();
    let mut ids = paths.keys().copied().collect::<Vec<_>>();
    ids.sort_unstable();

    for id in ids {
        let item = &paths[&id];

        if item.path.len() <= 1 {
            continue;
        }

        let parent_path = &item.path[..item.path.len() - 1];

        let Some(&parent_id) = path_to_id.get(parent_path) else {
            continue;
        };

        parents.insert(id, parent_id);
    }

    parents
}

fn path_to_id(paths: &Paths) -> HashMap<&[String], Id> {
    let mut path_to_id = HashMap::new();

    for (&id, item_summary) in paths {
        path_to_id.insert(item_summary.path.as_slice(), id);
    }

    path_to_id
}

fn item_name(item: &ItemSummary) -> &str {
    item.path.iter().last().map(|s| &**s).unwrap_or("")
}

fn item_kind(item: &ItemSummary) -> Option<Kind> {
    Some(match item.kind {
        ItemKind::Module => Kind::Module,
        ItemKind::ExternCrate => return None,
        ItemKind::Use => return None,
        ItemKind::Struct => Kind::Struct,
        ItemKind::StructField => Kind::StructField,
        ItemKind::Union => Kind::Union,
        ItemKind::Enum => Kind::Enum,
        ItemKind::Variant => Kind::Variant,
        ItemKind::Function => Kind::Function,
        ItemKind::TypeAlias => Kind::TypeAlias,
        ItemKind::Constant => Kind::Constant,
        ItemKind::Trait => Kind::Trait,
        ItemKind::TraitAlias => Kind::TraitAlias,
        ItemKind::Impl => return None,
        ItemKind::Static => Kind::Static,
        ItemKind::ExternType => Kind::ExternType,
        ItemKind::Macro => Kind::Macro,
        ItemKind::ProcAttribute => Kind::ProcAttribute,
        ItemKind::ProcDerive => Kind::ProcDerive,
        ItemKind::AssocConst => Kind::AssocConst,
        ItemKind::AssocType => Kind::AssocType,
        ItemKind::Primitive => Kind::Primitive,
        ItemKind::Keyword => return None,
    })
}
