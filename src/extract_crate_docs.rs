mod child_to_parent;

use std::{collections::HashMap, fs};

use cargo_metadata::PackageId;
use color_eyre::eyre::{Context as _, OptionExt as _, Report, Result, bail, eyre};
use rustdoc_types::{Crate, Id, Item, ItemEnum, ItemKind};
use serde::Deserialize;

use crate::{Context, markdown};

pub fn extract(cx: &Context) -> Result<String> {
    let crate_to_package: HashMap<String, &PackageId> =
        cx.metadata.packages.iter().map(|p| (p.name.as_ref().replace('-', "_"), &p.id)).collect();

    let krate = rustdoc_json(cx)?;
    let root = krate.index.get(&krate.root).ok_or_eyre("crate index has no root")?;
    let docs = root.docs.as_deref().unwrap_or("").to_string();

    let resolver = Resolve {
        cx,
        krate: &krate,
        child_to_parent: child_to_parent::child_to_parent(&krate)?.into_iter().collect(),
        path_to_kind: path_to_kind_map(&krate),
        crate_to_package,
    };

    let mut new_docs = docs.clone();

    for link in markdown::links(&docs).into_iter().rev() {
        let markdown::Link { span, link_type: _, dest_url, title, id: _, content_span } = link;

        let Some(&item_id) = root.links.get(&*dest_url) else {
            // rustdoc has no item for this url
            // the link could just not be rustdoc related like `https://www.rust-lang.org/` or
            // the link is dead because some feature is not enabled or code is cfg'd out
            // we keep such links as they are
            continue;
        };

        let url = match resolver.item_url(item_id) {
            Ok(ok) => Some(ok),
            Err(err) => {
                cx.log
                    .span("cause", err)
                    .span("link", &dest_url)
                    .warn("failed to resolve doc link");
                None
            }
        };

        let content = &docs[content_span.clone().unwrap_or(0..0)];

        let replace_with = match url {
            Some(mut url) => {
                // You can link to sections within an item's documentation by writing `[Vec](Vec#guarantees)`.
                if let Some(hash) = dest_url.find("#") {
                    url.push_str(&dest_url[hash..]);
                }

                use std::fmt::Write;
                let mut s = String::new();

                write!(s, "[{content}]({url}").unwrap();

                if !title.is_empty() {
                    write!(s, " \"{title}\"").unwrap();
                }

                write!(s, ")").unwrap();
                s
            }
            None => content.to_string(),
        };

        new_docs.replace_range(span, &replace_with);
    }

    let new_docs = markdown::clean_code_blocks(&new_docs);
    let new_docs = markdown::shrink_headings(&new_docs);

    Ok(new_docs)
}

fn rustdoc_json(cx: &Context) -> Result<Crate> {
    #[derive(Deserialize)]
    struct CrateWithJustTheFormatVersion {
        format_version: u32,
    }

    let mut builder = rustdoc_json::Builder::default()
        .toolchain(&cx.args.toolchain)
        .manifest_path(&cx.args.manifest_path)
        .all_features(cx.args.all_features)
        .no_default_features(cx.args.no_default_features)
        .features(&cx.package.enabled_features)
        .document_private_items(cx.args.document_private_items);

    if cx.package.is_explicit {
        builder = builder.package(&cx.package.name);
    }

    if let Some(target) = cx.args.target.as_ref() {
        builder = builder.target(target.to_string());
    }

    if cx.args.quiet_cargo {
        builder = builder.quiet(true).silent(true);
    } else {
        // write an empty line to separate our messages from the invoked command
        if cx.log.was_written_to() {
            cx.log.write('\n');
        }

        // the command invocation will write to stdout
        // setting this flag here will make the log insert a newline
        // before the next log message
        cx.log.set_written_to(true);
    }

    let json_path = builder.build()?;

    let json = fs::read_to_string(json_path).context("failed to read generated rustdoc json")?;

    let krate: CrateWithJustTheFormatVersion =
        serde_json::from_str(&json).context("failed to parse generated rustdoc json")?;

    if krate.format_version != rustdoc_types::FORMAT_VERSION {
        let expected = rustdoc_types::FORMAT_VERSION;
        let actual = krate.format_version;
        let what_to_do = if actual > expected {
            "update `cargo-insert-docs` or use an older nightly toolchain"
        } else {
            "upgrade your nightly toolchain"
        };

        bail!(
            "`cargo-insert-docs` requires rustdoc json format version {expected} \
            but rustdoc produced version {actual}\n\
            {what_to_do} to be able to use this tool"
        );
    }

    let krate: Crate =
        serde_json::from_str(&json).context("failed to parse generated rustdoc json")?;

    Ok(krate)
}

fn path_to_kind_map(krate: &Crate) -> HashMap<&[String], ItemKind> {
    let mut map = HashMap::new();

    for item_summary in krate.paths.values() {
        map.insert(&*item_summary.path, item_summary.kind);
    }

    map
}

struct Resolve<'a> {
    cx: &'a Context<'a>,
    krate: &'a Crate,
    child_to_parent: HashMap<Id, Id>,
    path_to_kind: HashMap<&'a [String], ItemKind>,
    crate_to_package: HashMap<String, &'a PackageId>,
}

impl Resolve<'_> {
    fn item_url(&self, id: Id) -> Result<String> {
        let path = self.item_path(id)?.into_iter().rev().collect();
        let path = fuse_impl_function_to_method(path);

        let mut url = String::new();

        // TODO: if url ends with / add index.html
        for (i, NameKind { name, kind }) in path.into_iter().enumerate() {
            let name = name.as_str();

            let segment = if i == 0 {
                // we expect this to be a crate
                self.crate_doc_url(name)
            } else {
                kind.to_path_segment(name)?
            };

            url.push_str(&segment);
        }

        Ok(url)
    }

    fn crate_doc_url(&self, name: &str) -> String {
        if matches!(name, "core" | "alloc" | "std") {
            format!("https://doc.rust-lang.org/{name}/")
        } else {
            let metadata = &self.cx.metadata;
            let package_id = self.crate_to_package.get(name);
            let package = package_id.map(|&p| &metadata[p]);
            let package_name = package.map(|p| p.name.as_str()).unwrap_or(name);
            let from_workspace = package_id.map(|&p| metadata.workspace_members.contains(p));
            let link_to_latest = self.cx.args.link_to_latest && from_workspace.unwrap_or(false);

            let version = if let Some(package) = package
                && !link_to_latest
            {
                package.version.to_string()
            } else {
                "latest".to_string()
            };

            format!("https://docs.rs/{package_name}/{version}/{name}/")
        }
    }

    fn item_path(&self, id: Id) -> Result<Vec<NameKind>> {
        let mut id = id;
        let mut item_path = vec![];

        loop {
            let item = self.item(id)?;

            item_path.push(NameKind { name: item.name, kind: item.kind });

            match item.parent {
                Some(Parent::Id(parent_id)) => {
                    id = parent_id;
                }
                Some(Parent::Path(parent_path)) => {
                    let mut path = parent_path;

                    loop {
                        let found = self.path_to_kind.get(&*path);

                        item_path.push(NameKind {
                            name: path.last().map(|x| &**x).unwrap_or("").to_string(),
                            kind: found.map(|&x| x.into()).unwrap_or(BasicItemKind::Module),
                        });

                        path.pop();

                        if path.is_empty() {
                            break;
                        }
                    }

                    break;
                }
                None => break,
            }
        }

        Ok(item_path)
    }

    fn item(&self, id: Id) -> Result<BasicItem> {
        if let Some(item) = self.krate.index.get(&id) {
            return Ok(BasicItem {
                name: item.name.as_deref().unwrap_or("").to_string(),
                kind: BasicItemKind::from(item),
                parent: self.child_to_parent.get(&id).copied().map(Parent::Id),
            });
        }

        if let Some(item_summary) = self.krate.paths.get(&id) {
            return Ok(BasicItem {
                name: item_summary.path.last().map(|x| x.as_str()).unwrap_or("").to_string(),
                kind: BasicItemKind::from(item_summary.kind),
                parent: pop(&item_summary.path).map(|x| x.to_vec()).map(Parent::Path),
            });
        }

        bail!("rustdoc produced dangling id?")
    }
}

fn pop<T>(slice: &[T]) -> Option<&[T]> {
    if slice.is_empty() { None } else { slice.get(..slice.len() - 1) }
}

fn fuse_impl_function_to_method(mut path: Vec<NameKind>) -> Vec<NameKind> {
    let index = path.windows(2).position(|item| {
        item.iter().map(|x| x.kind).eq([BasicItemKind::Impl, BasicItemKind::Function])
    });

    let Some(index) = index else {
        return path;
    };

    let name = path[index + 1].name.clone();

    path.splice(index..index + 2, [NameKind { name, kind: BasicItemKind::Method }]);

    path
}

struct NameKind {
    name: String,
    kind: BasicItemKind,
}

struct BasicItem {
    name: String,
    kind: BasicItemKind,
    parent: Option<Parent>,
}

enum Parent {
    Id(Id),
    Path(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BasicItemKind {
    // from Item
    Module,
    ExternCrate,
    Use,
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

    // from ItemSummary
    ProcAttribute,
    ProcDerive,
    Keyword,

    // custom
    Method,
}

impl BasicItemKind {
    fn to_path_segment(self, name: &str) -> Result<String> {
        Ok(match self {
            BasicItemKind::Module => format!("{name}/"),
            BasicItemKind::ExternCrate => {
                // Rustdoc doesn't link to an extern crate at the moment, just to an entry in `$.paths` of type `Module`.
                return Err(unexpected_path_segment(self, name));
            }
            BasicItemKind::Use => String::new(),
            BasicItemKind::Union => format!("union.{name}.html"),
            BasicItemKind::Struct => format!("struct.{name}.html"),
            BasicItemKind::StructField => format!("#structfield.{name}"),
            BasicItemKind::Enum => format!("enum.{name}.html"),
            BasicItemKind::Variant => format!("#variant.{name}"),
            BasicItemKind::Function => format!("fn.{name}.html"),
            BasicItemKind::Trait => format!("trait.{name}.html"),
            BasicItemKind::TraitAlias => format!("traitalias.{name}.html"),
            BasicItemKind::Impl => String::new(),
            BasicItemKind::TypeAlias => format!("type.{name}.html"),
            BasicItemKind::Constant => format!("constant.{name}.html"),
            BasicItemKind::Static => format!("static.{name}.html"),
            BasicItemKind::ExternType => format!("foreigntype.{name}.html"),
            BasicItemKind::Macro => format!("macro.{name}.html"),
            BasicItemKind::ProcMacro => format!("macro.{name}.html"),
            BasicItemKind::Primitive => format!("primitive.{name}.html"),
            BasicItemKind::AssocConst => format!("#associatedconstant.{name}"),
            BasicItemKind::AssocType => format!("#associatedtype.{name}"),
            BasicItemKind::ProcAttribute => format!("attr.{name}.html"),
            BasicItemKind::ProcDerive => format!("derive.{name}.html"),
            BasicItemKind::Keyword => {
                return Err(unexpected_path_segment(self, name));
            }
            BasicItemKind::Method => format!("#method.{name}"),
        })
    }
}

fn unexpected_path_segment(kind: BasicItemKind, name: &str) -> Report {
    eyre!(
        "encountered unexpected url path segment '{kind:?}' with the name '{name}'\n\
            This is a bug! please report it at:\n\
            https://github.com/bluurryy/cargo-insert-docs\
        "
    )
}

#[allow(clippy::unneeded_struct_pattern)]
impl From<&Item> for BasicItemKind {
    fn from(value: &Item) -> Self {
        match &value.inner {
            ItemEnum::Module { .. } => BasicItemKind::Module,
            ItemEnum::ExternCrate { .. } => BasicItemKind::ExternCrate,
            ItemEnum::Use { .. } => BasicItemKind::Use,
            ItemEnum::Union { .. } => BasicItemKind::Union,
            ItemEnum::Struct { .. } => BasicItemKind::Struct,
            ItemEnum::StructField { .. } => BasicItemKind::StructField,
            ItemEnum::Enum { .. } => BasicItemKind::Enum,
            ItemEnum::Variant { .. } => BasicItemKind::Variant,
            ItemEnum::Function { .. } => BasicItemKind::Function,
            ItemEnum::Trait { .. } => BasicItemKind::Trait,
            ItemEnum::TraitAlias { .. } => BasicItemKind::TraitAlias,
            ItemEnum::Impl { .. } => BasicItemKind::Impl,
            ItemEnum::TypeAlias { .. } => BasicItemKind::TypeAlias,
            ItemEnum::Constant { .. } => BasicItemKind::Constant,
            ItemEnum::Static { .. } => BasicItemKind::Static,
            ItemEnum::ExternType { .. } => BasicItemKind::ExternType,
            ItemEnum::Macro { .. } => BasicItemKind::Macro,
            ItemEnum::ProcMacro { .. } => BasicItemKind::ProcMacro,
            ItemEnum::Primitive { .. } => BasicItemKind::Primitive,
            ItemEnum::AssocConst { .. } => BasicItemKind::AssocConst,
            ItemEnum::AssocType { .. } => BasicItemKind::AssocType,
        }
    }
}

impl From<ItemKind> for BasicItemKind {
    fn from(value: ItemKind) -> Self {
        match value {
            ItemKind::Module => BasicItemKind::Module,
            ItemKind::ExternCrate => BasicItemKind::ExternCrate,
            ItemKind::Use => BasicItemKind::Use,
            ItemKind::Struct => BasicItemKind::Struct,
            ItemKind::StructField => BasicItemKind::StructField,
            ItemKind::Union => BasicItemKind::Union,
            ItemKind::Enum => BasicItemKind::Enum,
            ItemKind::Variant => BasicItemKind::Variant,
            ItemKind::Function => BasicItemKind::Function,
            ItemKind::TypeAlias => BasicItemKind::TypeAlias,
            ItemKind::Constant => BasicItemKind::Constant,
            ItemKind::Trait => BasicItemKind::Trait,
            ItemKind::TraitAlias => BasicItemKind::TraitAlias,
            ItemKind::Impl => BasicItemKind::Impl,
            ItemKind::Static => BasicItemKind::Static,
            ItemKind::ExternType => BasicItemKind::ExternType,
            ItemKind::Macro => BasicItemKind::Macro,
            ItemKind::ProcAttribute => BasicItemKind::ProcAttribute,
            ItemKind::ProcDerive => BasicItemKind::ProcDerive,
            ItemKind::AssocConst => BasicItemKind::AssocConst,
            ItemKind::AssocType => BasicItemKind::AssocType,
            ItemKind::Primitive => BasicItemKind::Primitive,
            ItemKind::Keyword => BasicItemKind::Keyword,
        }
    }
}
