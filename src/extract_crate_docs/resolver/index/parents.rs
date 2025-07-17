//! Resolves item parents. (from `.index`)
//!
//! We need to choose what parent to choose:
//! - the shortest path for items and items available through a `#[doc(inline)] use ...`
//! - don't choose non-`#[doc(inline)]` `use`s unless its the only path

use std::collections::{HashMap, hash_map::Entry};

use color_eyre::eyre::{Result, bail};
use rustdoc_types::Id;
use tracing::error_span;

use super::simple::{SimpleItem, SimpleItemKind};

const RECURSION_LIMIT: usize = 64;

pub fn parents(index: &HashMap<Id, SimpleItem>, root: Id) -> Result<HashMap<Id, Id>> {
    let mut parents = HashMap::new();
    parents_recurse(index, &mut parents, root, 0, &mut vec![])?;
    Ok(parents.into_iter().map(|(child_id, parent)| (child_id, parent.id)).collect())
}

fn parents_recurse<'a>(
    index: &HashMap<Id, SimpleItem<'a>>,
    parents: &mut HashMap<Id, Parent>,
    parent_id: Id,
    depth: usize,
    path_for_error: &mut Vec<&'a str>,
) -> Result<()> {
    if path_for_error.len() > RECURSION_LIMIT {
        let item_path = path_for_error
            .iter()
            .filter_map(|&s| (!s.is_empty()).then_some(s))
            .collect::<Vec<_>>()
            .join("::");

        let _span = error_span!("", %item_path).entered();
        bail!("recursed too deep while resolving item paths ({RECURSION_LIMIT})");
    }

    let Some(parent_item) = index.get(&parent_id) else {
        return Ok(());
    };

    path_for_error.push(parent_item.name);

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

        parents_recurse(index, parents, child_id, child_depth, path_for_error)?;
        path_for_error.pop();
    }

    Ok(())
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
