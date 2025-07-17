//! Parses `.index` into a simpler representation fitting our use case.

use rustdoc_types::{Crate, Id, Item, ItemEnum, StructKind, VariantKind};

pub struct SimpleItem<'a> {
    pub name: &'a str,
    pub kind: SimpleItemKind,
    pub children: Vec<Id>,
}

impl<'a> SimpleItem<'a> {
    pub fn from_item(krate: &'a Crate, item: &'a Item) -> Self {
        Self { name: name(item), kind: kind(item), children: children(krate, item) }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SimpleItemKind {
    Module,
    ExternCrate,
    Use { inline: bool },
    Union,
    Struct,
    StructField,
    Enum,
    Variant,
    Function,
    Trait,
    TraitAlias,
    Impl,
    TypeAlias,
    Constant,
    Static,
    ExternType,
    Macro,
    ProcMacro,
    Primitive,
    AssocConst,
    AssocType,
}

fn name(item: &Item) -> &str {
    item.name.as_deref().unwrap_or("")
}

fn kind(item: &Item) -> SimpleItemKind {
    #[expect(clippy::unneeded_struct_pattern)]
    match &item.inner {
        ItemEnum::Module { .. } => SimpleItemKind::Module,
        ItemEnum::ExternCrate { .. } => SimpleItemKind::ExternCrate,
        ItemEnum::Use { .. } => SimpleItemKind::Use { inline: is_doc_inline(item) },
        ItemEnum::Union { .. } => SimpleItemKind::Union,
        ItemEnum::Struct { .. } => SimpleItemKind::Struct,
        ItemEnum::StructField { .. } => SimpleItemKind::StructField,
        ItemEnum::Enum { .. } => SimpleItemKind::Enum,
        ItemEnum::Variant { .. } => SimpleItemKind::Variant,
        ItemEnum::Function { .. } => SimpleItemKind::Function,
        ItemEnum::Trait { .. } => SimpleItemKind::Trait,
        ItemEnum::TraitAlias { .. } => SimpleItemKind::TraitAlias,
        ItemEnum::Impl { .. } => SimpleItemKind::Impl,
        ItemEnum::TypeAlias { .. } => SimpleItemKind::TypeAlias,
        ItemEnum::Constant { .. } => SimpleItemKind::Constant,
        ItemEnum::Static { .. } => SimpleItemKind::Static,
        ItemEnum::ExternType { .. } => SimpleItemKind::ExternType,
        ItemEnum::Macro { .. } => SimpleItemKind::Macro,
        ItemEnum::ProcMacro { .. } => SimpleItemKind::ProcMacro,
        ItemEnum::Primitive { .. } => SimpleItemKind::Primitive,
        ItemEnum::AssocConst { .. } => SimpleItemKind::AssocConst,
        ItemEnum::AssocType { .. } => SimpleItemKind::AssocType,
    }
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
pub fn children(krate: &Crate, item: &Item) -> Vec<Id> {
    match &item.inner {
        ItemEnum::Module(inner) => chain!(&inner.items),
        ItemEnum::ExternCrate { .. } => chain!(),
        ItemEnum::Use(inner) => {
            if inner.is_glob {
                inner
                    .id
                    .and_then(|id| krate.index.get(&id))
                    .into_iter()
                    .flat_map(|item| children(krate, item))
                    .collect()
            } else {
                chain!(&inner.id)
            }
        }
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

fn is_doc_inline(item: &Item) -> bool {
    for attr_str in &item.attrs {
        if let Ok(attr) = parse_attr_str(attr_str)
            && attr.path().is_ident("doc")
            && let syn::Meta::List(list) = attr.meta
        {
            for token in list.tokens {
                if token.to_string() == "inline" {
                    return true;
                }
            }
        }
    }

    false
}

/// `Attribute` does not implement `Parse` (WHY NOT?) so we need to do it ourselves.
fn parse_attr_str(str: &str) -> syn::Result<syn::Attribute> {
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
