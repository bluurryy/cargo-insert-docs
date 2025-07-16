//! Resolves item parents. (from `.index`)
//!
//! We need to choose what parent to choose:
//! - the shortest path for items and items available through a `#[doc(inline)] use ...`
//! - don't choose non-`#[doc(inline)]` `use`s unless its the only path

use std::collections::{HashMap, hash_map::Entry};

use rustdoc_types::Id;

use super::simple::{SimpleItem, SimpleItemKind};

pub fn parents(index: &HashMap<Id, SimpleItem>, root: Id) -> HashMap<Id, Id> {
    let mut parents = HashMap::new();
    parents_recurse(index, &mut parents, root, 0);
    parents.into_iter().map(|(child_id, parent)| (child_id, parent.id)).collect()
}

fn parents_recurse(
    index: &HashMap<Id, SimpleItem>,
    parents: &mut HashMap<Id, Parent>,
    parent_id: Id,
    depth: usize,
) {
    let Some(parent_item) = index.get(&parent_id) else {
        return;
    };

    let parent_is_use = matches!(&parent_item.kind, SimpleItemKind::Use { .. });

    let parent = Parent {
        id: parent_id,
        depth,
        kind: if matches!(parent_item.kind, SimpleItemKind::Use { inline: false }) {
            ParentKind::NonInlineUse
        } else {
            ParentKind::Other
        },
    };

    for &child_id in &parent_item.children {
        let Some(child_item) = index.get(&child_id) else {
            continue;
        };

        match parents.entry(child_id) {
            Entry::Occupied(mut entry) => {
                if parent.is_better_than(entry.get()) {
                    entry.insert(parent);
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(parent);
            }
        }

        let child_is_use = matches!(&child_item.kind, SimpleItemKind::Use { .. });

        // A `use` does not count towards depth.
        let child_depth = if parent_is_use || child_is_use { depth } else { depth + 1 };

        parents_recurse(index, parents, child_id, child_depth);
    }
}

#[derive(Clone, Copy)]
struct Parent {
    id: Id,
    // Smaller is better.
    depth: usize,
    kind: ParentKind,
}

impl Parent {
    fn is_better_than(&self, other: &Parent) -> bool {
        // `NonInlineUse`s are to be avoided
        if self.kind == ParentKind::NonInlineUse || other.kind == ParentKind::NonInlineUse {
            return other.kind == ParentKind::NonInlineUse;
        }

        self.depth < other.depth
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ParentKind {
    /// We should avoid non-inline uses unless there is no other path.
    /// The html backend implicitly inlines such cases.
    NonInlineUse,
    Other,
}
