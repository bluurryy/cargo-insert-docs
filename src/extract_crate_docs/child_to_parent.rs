#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, btree_map::Entry};

use color_eyre::eyre::{Context as _, Result};
use rustdoc_types::{Crate, Id, Item, ItemEnum, StructKind, VariantKind, Visibility};

pub fn child_to_parent(krate: &Crate) -> Result<BTreeMap<Id, Id>> {
    Ok(clean_map(krate, &child_to_parent_struct(krate)?))
}

#[derive(Debug, Clone, Copy)]
struct Parent {
    id: Id,
    // a score which we use to choose parents with a lower depth
    depth: usize,
    // we should only use non-inline uses if there is no other path
    // (such cases are implicitly inlined in the html docs)
    is_non_inline_use: bool,
}

impl Parent {
    fn is_better_than(&self, other: &Parent) -> bool {
        // we should not resolve to non inlined uses
        if self.is_non_inline_use || other.is_non_inline_use {
            return other.is_non_inline_use;
        }

        self.depth < other.depth
    }
}

// removes uses
fn clean_map(krate: &Crate, child_to_parent_struct: &BTreeMap<Id, Parent>) -> BTreeMap<Id, Id> {
    let mut child_to_parent = BTreeMap::new();

    for (&child_id, parent) in child_to_parent_struct {
        let mut parent_id = parent.id;

        if matches!(krate.index[&child_id].inner, ItemEnum::Use { .. }) {
            continue;
        }

        loop {
            if matches!(krate.index[&parent_id].inner, ItemEnum::Use { .. })
                && let Some(grand_parent) = child_to_parent_struct.get(&parent_id)
            {
                parent_id = grand_parent.id;
            } else {
                child_to_parent.insert(child_id, parent_id);
                break;
            }
        }
    }

    child_to_parent
}

fn child_to_parent_struct(krate: &Crate) -> Result<BTreeMap<Id, Parent>> {
    let mut map = BTreeMap::new();
    child_to_parent_struct_build(krate, krate.root, &mut map, 0)?;
    Ok(map)
}

fn child_to_parent_struct_build(
    krate: &Crate,
    parent_id: Id,
    map: &mut BTreeMap<Id, Parent>,
    depth: usize,
) -> Result<()> {
    macro_rules! is {
        ($item:expr, $kind:ident) => {
            matches!($item.inner, ItemEnum::$kind { .. })
        };
    }

    let Some(parent_item) = krate.index.get(&parent_id) else {
        return Ok(());
    };

    let is_trait = is!(parent_item, Trait);
    let is_use = is!(parent_item, Use);

    let parent = Parent {
        id: parent_id,
        depth,
        is_non_inline_use: is_use && !doc_attributes(parent_item)?.inline,
    };

    for child_id in children(parent_item) {
        let Some(child_item) = krate.index.get(&child_id) else {
            continue;
        };

        let is_public = match child_item.visibility {
            Visibility::Public => true,
            Visibility::Default => is_trait,
            _ => false,
        };

        if !is_public {
            // continue;
        }

        match map.entry(child_id) {
            Entry::Occupied(mut entry) => {
                if parent.is_better_than(entry.get()) {
                    entry.insert(parent);
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(parent);
            }
        }

        // a use does not count towards the depth score
        let new_depth = if is_use || is!(child_item, Use) { depth } else { depth + 1 };

        child_to_parent_struct_build(krate, child_id, map, new_depth)?;
    }

    Ok(())
}

struct DocAttributes {
    inline: bool,
}

/// syn does not implement `Parse` for `Attribute` (WHY?)
fn syn_parse_attr_str(str: &str) -> syn::Result<syn::Attribute> {
    struct Helper(syn::Attribute);

    impl syn::parse::Parse for Helper {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let pound_token = input.parse()?;

            let style = if input.peek(syn::Token![!]) {
                syn::AttrStyle::Inner(input.parse()?)
            } else {
                syn::AttrStyle::Outer
            };

            let content;

            Ok(Helper(syn::Attribute {
                pound_token,
                style,
                bracket_token: syn::bracketed!(content in input),
                meta: content.parse()?,
            }))
        }
    }

    Ok(syn::parse_str::<Helper>(str)?.0)
}

fn doc_attributes(item: &Item) -> Result<DocAttributes> {
    let mut doc = DocAttributes { inline: false };

    for attr in &item.attrs {
        if let Ok(attr) = syn_parse_attr_str(attr).wrap_err("failed to parse attribute")
            && attr.path().is_ident("doc")
            && let syn::Meta::List(list) = attr.meta
        {
            for token in list.tokens {
                if token.to_string() == "inline" {
                    doc.inline = true;
                }
            }
        }
    }

    Ok(doc)
}

macro_rules! chain {
    () => { vec![] };
    ($expr:expr $(,$rest:expr)*) => {
        chain!(@inner $expr $(, $rest)*).collect()
    };
    (@inner $expr:expr $(, $rest:expr)+) => {
        $expr.into_iter().copied().chain(chain!(@inner $($rest),+))
    };
    (@inner $expr:expr) => {
        $expr.into_iter().copied()
    };
}

#[allow(clippy::unneeded_struct_pattern)]
fn children(item: &Item) -> Vec<Id> {
    match &item.inner {
        ItemEnum::Module(inner) => chain!(&inner.items),
        ItemEnum::ExternCrate { .. } => chain!(),
        ItemEnum::Use(inner) => chain!(&inner.id),
        ItemEnum::Union(inner) => chain!(&inner.fields, &inner.impls),
        ItemEnum::Struct(inner) => match &inner.kind {
            StructKind::Unit => chain!(),
            StructKind::Tuple(ids) => chain!(ids.iter().filter_map(Option::as_ref), &inner.impls),
            StructKind::Plain { fields, .. } => chain!(fields, &inner.impls),
        },
        ItemEnum::StructField { .. } => chain!(),
        ItemEnum::Enum(inner) => chain!(&inner.variants, &inner.impls),
        ItemEnum::Variant(inner) => match &inner.kind {
            VariantKind::Plain => chain!(),
            VariantKind::Tuple(ids) => chain!(ids.iter().filter_map(Option::as_ref)),
            VariantKind::Struct { fields, .. } => chain!(fields),
        },
        ItemEnum::Function { .. } => chain!(),
        ItemEnum::Trait(inner) => chain!(&inner.items, &inner.implementations),
        ItemEnum::TraitAlias { .. } => chain!(),
        ItemEnum::Impl(inner) => chain!(&inner.items),
        ItemEnum::TypeAlias { .. } => chain!(),
        ItemEnum::Constant { .. } => chain!(),
        ItemEnum::Static { .. } => chain!(),
        ItemEnum::ExternType { .. } => chain!(),
        ItemEnum::Macro { .. } => chain!(),
        ItemEnum::ProcMacro { .. } => chain!(),
        ItemEnum::Primitive(inner) => chain!(&inner.impls),
        ItemEnum::AssocConst { .. } => chain!(),
        ItemEnum::AssocType { .. } => chain!(),
    }
}
