use std::collections::HashMap;

use cargo_metadata::{Metadata, PackageId};
use color_eyre::eyre::{Result, bail};
use rustdoc_types::{Crate, Id};

mod index;
mod paths;

pub struct Resolver<'a> {
    metadata: &'a Metadata,
    index: index::Tree<'a>,
    paths: paths::Tree<'a>,
    crate_to_package: HashMap<String, &'a PackageId>,
    options: &'a ResolverOptions,
}

pub struct ResolverOptions {
    pub link_to_latest: bool,
}

impl<'a> Resolver<'a> {
    pub fn new(krate: &'a Crate, metadata: &'a Metadata, options: &'a ResolverOptions) -> Self {
        Self {
            metadata,
            index: index::Tree::new(krate),
            paths: paths::Tree::new(krate),
            crate_to_package: metadata
                .packages
                .iter()
                .map(|p| (p.name.as_ref().replace('-', "_"), &p.id))
                .collect(),
            options,
        }
    }

    pub fn item_url(&self, id: Id) -> Result<String> {
        let path = self.item_path(id)?;
        let mut url = String::new();

        for (i, item) in path.iter().rev().enumerate() {
            url.push_str(&if i == 0 {
                // The first item in a path ought to be a crate.
                self.crate_doc_url(item.name)
            } else {
                item.url_path_segment()
            });
        }

        if url.ends_with('/') {
            url.push_str("index.html");
        }

        Ok(url)
    }

    fn item_path(&self, id: Id) -> Result<Vec<PathItem<'a>>> {
        if let Some(path) = self.index.path_to(id) {
            return Ok(path);
        }

        if let Some(path) = self.paths.path_to(id) {
            return Ok(path);
        }

        // Expected to happen, for example when referring to a method of another crate.
        // See <https://github.com/rust-lang/rust/issues?q=state%3Aopen%20label%3AA-rustdoc-json%20paths>.
        bail!("rustdoc produced dangling id (known bug of rustdoc)")
    }

    fn crate_doc_url(&self, name: &str) -> String {
        if matches!(name, "core" | "alloc" | "std") {
            format!("https://doc.rust-lang.org/{name}/")
        } else {
            let metadata = &self.metadata;
            let package_id = self.crate_to_package.get(name);
            let package = package_id.map(|&p| &metadata[p]);
            let package_name = package.map(|p| p.name.as_str()).unwrap_or(name);
            let from_workspace = package_id.map(|&p| metadata.workspace_members.contains(p));
            let link_to_latest = self.options.link_to_latest && from_workspace.unwrap_or(false);

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
}

#[derive(Debug)]
struct PathItem<'a> {
    name: &'a str,
    kind: Kind,
}

impl<'a> PathItem<'a> {
    fn url_path_segment(&self) -> String {
        let Self { name, kind } = *self;

        match kind {
            Kind::Module => format!("{name}/"),
            Kind::Union => format!("union.{name}.html"),
            Kind::Struct => format!("struct.{name}.html"),
            Kind::StructField => format!("#structfield.{name}"),
            Kind::Enum => format!("enum.{name}.html"),
            Kind::Variant => format!("#variant.{name}"),
            Kind::Function => format!("fn.{name}.html"),
            Kind::Trait => format!("trait.{name}.html"),
            Kind::TraitAlias => format!("traitalias.{name}.html"),
            Kind::TypeAlias => format!("type.{name}.html"),
            Kind::Constant => format!("constant.{name}.html"),
            Kind::Static => format!("static.{name}.html"),
            Kind::ExternType => format!("foreigntype.{name}.html"),
            Kind::Macro => format!("macro.{name}.html"),
            Kind::ProcMacro => format!("macro.{name}.html"),
            Kind::Primitive => format!("primitive.{name}.html"),
            Kind::AssocConst => format!("#associatedconstant.{name}"),
            Kind::AssocType => format!("#associatedtype.{name}"),
            Kind::ProcAttribute => format!("attr.{name}.html"),
            Kind::ProcDerive => format!("derive.{name}.html"),
            Kind::Method => format!("#method.{name}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Module,
    Union,
    Struct,
    StructField,
    Enum,
    Variant,
    Function,
    Trait,
    TraitAlias,
    TypeAlias,
    Constant,
    Static,
    ExternType,
    Macro,
    ProcMacro,
    Primitive,
    AssocConst,
    AssocType,
    ProcAttribute,
    ProcDerive,

    /// This type doesn't come from rustdoc json directly.
    ///
    /// We infer [`Function`](Kind::Function)s to be [`Method`](Kind::Method)s when
    /// they're inside an [`rustdoc_types::ItemEnum::Impl`] or [`rustdoc_types::ItemKind::Impl`].
    Method,
}
