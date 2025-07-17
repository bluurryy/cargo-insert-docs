//! Processes `.index`.

mod parents;
mod simple;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use color_eyre::eyre::Result;
use rustdoc_types::{Crate, Id};

use super::{Kind, PathItem, index::simple::SimpleItemKind};

use simple::SimpleItem;

pub struct Tree<'a> {
    inv_tree: HashMap<Id, Value<'a>>,
}

impl<'a> Tree<'a> {
    pub fn new(krate: &'a Crate) -> Result<Self> {
        let index =
            krate.index.iter().map(|(k, v)| (*k, SimpleItem::from_item(krate, v))).collect();
        Self::new_simple(&index, krate.root)
    }

    fn new_simple(index: &HashMap<Id, SimpleItem<'a>>, root: Id) -> Result<Self> {
        let parents = parents::parents(index, root)?;
        let mut inv_tree = HashMap::new();

        for &child_id in index.keys() {
            let child_item = &index[&child_id];

            // We skip `Use` and `Impl`s because we dissolve them
            // (remove them and link the child to the grandparent)
            // in the loop below.
            //
            // The `ExternCrate`... I have no idea if or where that would
            // come up and how it would be handled so for now we filter that out.
            let Some(mut child_kind) = item_kind(&child_item.kind) else {
                continue;
            };

            let parent_id = {
                if let Some(&(mut parent_id)) = parents.get(&child_id) {
                    loop {
                        let parent_item = &index[&parent_id];

                        if matches!(
                            parent_item.kind,
                            SimpleItemKind::Use { .. } | SimpleItemKind::Impl
                        ) && let Some(&grand_parent_id) = parents.get(&parent_id)
                        {
                            parent_id = grand_parent_id;
                        }

                        if matches!(parent_item.kind, SimpleItemKind::Use { .. }) {
                            continue;
                        }

                        if matches!(parent_item.kind, SimpleItemKind::Impl)
                            && matches!(child_kind, Kind::Function)
                        {
                            child_kind = Kind::Method;
                        }

                        break Some(parent_id);
                    }
                } else {
                    None
                }
            };

            inv_tree.insert(
                child_id,
                Value { parent: parent_id, kind: child_kind, name: child_item.name },
            );
        }

        Ok(Self { inv_tree })
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

        Some(path)
    }
}

#[derive(Clone, Copy)]
struct Value<'a> {
    parent: Option<Id>,
    kind: Kind,
    name: &'a str,
}

#[allow(clippy::unneeded_struct_pattern)]
fn item_kind(item: &SimpleItemKind) -> Option<Kind> {
    Some(match *item {
        SimpleItemKind::Module { .. } => Kind::Module,
        SimpleItemKind::ExternCrate { .. } => return None,
        SimpleItemKind::Use { .. } => return None,
        SimpleItemKind::Union { .. } => Kind::Union,
        SimpleItemKind::Struct { .. } => Kind::Struct,
        SimpleItemKind::StructField { .. } => Kind::StructField,
        SimpleItemKind::Enum { .. } => Kind::Enum,
        SimpleItemKind::Variant { .. } => Kind::Variant,
        SimpleItemKind::Function { .. } => Kind::Function,
        SimpleItemKind::Trait { .. } => Kind::Trait,
        SimpleItemKind::TraitAlias { .. } => Kind::TraitAlias,
        SimpleItemKind::Impl { .. } => return None,
        SimpleItemKind::TypeAlias { .. } => Kind::TypeAlias,
        SimpleItemKind::Constant { .. } => Kind::Constant,
        SimpleItemKind::Static { .. } => Kind::Static,
        SimpleItemKind::ExternType { .. } => Kind::ExternType,
        SimpleItemKind::Macro { .. } => Kind::Macro,
        SimpleItemKind::ProcMacro { .. } => Kind::ProcMacro,
        SimpleItemKind::Primitive { .. } => Kind::Primitive,
        SimpleItemKind::AssocConst { .. } => Kind::AssocConst,
        SimpleItemKind::AssocType { .. } => Kind::AssocType,
    })
}
