#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
};

use anstream::ColorChoice;
use color_eyre::eyre::{Result, WrapErr as _};
use macro_rules_attribute::derive;
use serde::{
    Deserialize, Serialize, Serializer,
    de::{DeserializeOwned, IgnoredAny},
};

pub const DEFAULT_FEATURE_LABEL: &str = "**`{feature}`**";
pub const DEFAULT_FEATURE_SECTION_NAME: &str = "feature documentation";
pub const DEFAULT_CRATE_SECTION_NAME: &str = "crate documentation";
pub const DEFAULT_TOOLCHAIN: &str = "nightly-2025-12-05";
pub const DEFAULT_SHRINK_HEADINGS: i8 = 1;

macro_rules! Fields {
    (
        $(#[$meta:meta])*
        $vis:vis struct $ident:ident {
            $($field_vis:vis $field:ident: $field_ty:ty),* $(,)?
        }
    ) => {
        impl $ident {
            const FIELDS: &[&str] = &[
                $(stringify!($field),)*
            ];
        }
    };
}

pub struct CliConfig {
    pub print_supported_toolchain: bool,
    pub print_config: bool,
    pub color: ColorChoice,
    pub verbose: u8,
    pub quiet: bool,
    pub quiet_cargo: bool,
    pub manifest_path: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct WorkspaceConfig {
    pub package: Vec<String>,
    pub workspace: bool,
    pub exclude: Vec<String>,
}

pub fn read_workspace_config(
    json: &serde_json::Value,
) -> Result<(WorkspaceConfigPatch, PackageConfigPatch)> {
    let wrk: WorkspaceConfigPatch = metadata_json(json)?;
    let pkg: PackageConfigPatch = metadata_json(json)?;
    let fields: HashMap<String, IgnoredAny> = metadata_json(json)?;
    warn_about_unused_fields(fields, &[WorkspaceConfigPatch::FIELDS, PackageConfigPatch::FIELDS]);
    Ok((wrk, pkg))
}

pub fn read_package_config(toml: &str) -> Result<PackageConfigPatch> {
    let pkg: PackageConfigPatch = metadata_toml(toml)?;
    let fields: HashMap<String, IgnoredAny> = metadata_toml(toml)?;
    warn_about_unused_fields(fields, &[PackageConfigPatch::FIELDS]);
    Ok(pkg)
}

#[derive(Default, Clone, Deserialize, Serialize, Fields!)]
#[serde(default, rename_all = "kebab-case")]
pub struct WorkspaceConfigPatch {
    pub package: Option<Vec<String>>,
    pub workspace: Option<bool>,
    pub exclude: Option<Vec<String>>,
}

impl WorkspaceConfigPatch {
    pub fn apply(&self, overwrite: &Self) -> Self {
        let mut this = self.clone();

        if let Some(package) = &overwrite.package {
            this.package = Some(package.clone());
        }
        if let Some(workspace) = overwrite.workspace {
            this.workspace = Some(workspace);
        }
        if let Some(exclude) = &overwrite.exclude {
            this.exclude = Some(exclude.clone());
        }

        this
    }

    pub fn finish(self) -> WorkspaceConfig {
        let Self { package, workspace, exclude } = self;
        WorkspaceConfig {
            package: package.unwrap_or_default(),
            workspace: workspace.unwrap_or_default(),
            exclude: exclude.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PackageConfig {
    pub feature_into_crate: bool,
    pub crate_into_readme: bool,
    pub feature_label: String,
    pub feature_section_name: String,
    pub crate_section_name: String,
    pub shrink_headings: i8,
    pub link_to_latest: bool,
    pub document_private_items: bool,
    pub no_deps: bool,
    pub check: bool,
    pub allow_missing_section: bool,
    pub allow_dirty: bool,
    pub allow_staged: bool,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    #[serde(flatten, serialize_with = "serialize_target_selection")]
    pub target_selection: Option<TargetSelection>,
    pub toolchain: String,
    pub target: Option<String>,
    pub target_dir: Option<PathBuf>,
    pub readme_path: Option<PathBuf>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Fields!)]
#[serde(default, rename_all = "kebab-case")]
pub struct PackageConfigPatch {
    pub feature_into_crate: Option<bool>,
    pub crate_into_readme: Option<bool>,
    pub feature_label: Option<String>,
    pub feature_section_name: Option<String>,
    pub crate_section_name: Option<String>,
    pub shrink_headings: Option<i8>,
    pub link_to_latest: Option<bool>,
    pub document_private_items: Option<bool>,
    pub no_deps: Option<bool>,
    pub check: Option<bool>,
    pub allow_missing_section: Option<bool>,
    pub allow_dirty: Option<bool>,
    pub allow_staged: Option<bool>,
    pub features: Option<Vec<String>>,
    pub all_features: Option<bool>,
    pub no_default_features: Option<bool>,
    pub lib: Option<bool>,
    pub bin: Option<BoolOrString>,
    pub toolchain: Option<String>,
    pub target: Option<String>,
    pub target_dir: Option<PathBuf>,
    pub readme_path: Option<PathBuf>,
}

impl PackageConfigPatch {
    pub fn apply(&self, overwrite: &Self) -> Self {
        let mut this = self.clone();

        if let Some(feature_into_crate) = overwrite.feature_into_crate {
            this.feature_into_crate = Some(feature_into_crate);
        }
        if let Some(crate_into_readme) = overwrite.crate_into_readme {
            this.crate_into_readme = Some(crate_into_readme);
        }
        if let Some(feature_label) = &overwrite.feature_label {
            this.feature_label = Some(feature_label.clone());
        }
        if let Some(feature_section_name) = &overwrite.feature_section_name {
            this.feature_section_name = Some(feature_section_name.clone());
        }
        if let Some(crate_section_name) = &overwrite.crate_section_name {
            this.crate_section_name = Some(crate_section_name.clone());
        }
        if let Some(shrink_headings) = overwrite.shrink_headings {
            this.shrink_headings = Some(shrink_headings);
        }
        if let Some(link_to_latest) = overwrite.link_to_latest {
            this.link_to_latest = Some(link_to_latest);
        }
        if let Some(document_private_items) = overwrite.document_private_items {
            this.document_private_items = Some(document_private_items);
        }
        if let Some(no_deps) = overwrite.no_deps {
            this.no_deps = Some(no_deps);
        }
        if let Some(check) = overwrite.check {
            this.check = Some(check);
        }
        if let Some(allow_missing_section) = overwrite.allow_missing_section {
            this.allow_missing_section = Some(allow_missing_section);
        }
        if let Some(allow_dirty) = overwrite.allow_dirty {
            this.allow_dirty = Some(allow_dirty);
        }
        if let Some(allow_staged) = overwrite.allow_staged {
            this.allow_staged = Some(allow_staged);
        }
        if let Some(features) = &overwrite.features {
            this.features = Some(features.clone());
        }
        if let Some(all_features) = overwrite.all_features {
            this.all_features = Some(all_features);
        }
        if let Some(no_default_features) = overwrite.no_default_features {
            this.no_default_features = Some(no_default_features);
        }
        if overwrite.lib.is_some() || overwrite.bin.is_some() {
            this.lib = overwrite.lib;
            this.bin = overwrite.bin.clone();
        }
        if let Some(toolchain) = &overwrite.toolchain {
            this.toolchain = Some(toolchain.clone());
        }
        if let Some(target) = &overwrite.target {
            this.target = Some(target.clone());
        }
        if let Some(target_dir) = &overwrite.target_dir {
            this.target_dir = Some(target_dir.clone());
        }
        if let Some(readme_path) = &overwrite.readme_path {
            this.readme_path = Some(readme_path.clone());
        }

        this
    }

    pub fn finish(self) -> PackageConfig {
        let PackageConfigPatch {
            feature_into_crate,
            crate_into_readme,
            feature_label,
            feature_section_name,
            crate_section_name,
            shrink_headings,
            link_to_latest,
            document_private_items,
            no_deps,
            check,
            allow_missing_section,
            allow_dirty,
            allow_staged,
            features,
            all_features,
            no_default_features,
            toolchain,
            lib,
            bin,
            target,
            target_dir,
            readme_path,
        } = self;

        PackageConfig {
            feature_into_crate: feature_into_crate.unwrap_or(true),
            crate_into_readme: crate_into_readme.unwrap_or(true),
            feature_label: feature_label.unwrap_or_else(|| DEFAULT_FEATURE_LABEL.to_string()),
            feature_section_name: feature_section_name
                .unwrap_or_else(|| DEFAULT_FEATURE_SECTION_NAME.to_string()),
            crate_section_name: crate_section_name
                .unwrap_or_else(|| DEFAULT_CRATE_SECTION_NAME.to_string()),
            shrink_headings: shrink_headings.unwrap_or(DEFAULT_SHRINK_HEADINGS),
            link_to_latest: link_to_latest.unwrap_or_default(),
            document_private_items: document_private_items.unwrap_or_default(),
            no_deps: no_deps.unwrap_or_default(),
            check: check.unwrap_or_default(),
            allow_missing_section: allow_missing_section.unwrap_or_default(),
            allow_dirty: allow_dirty.unwrap_or_default(),
            allow_staged: allow_dirty.or(allow_staged).unwrap_or_default(),
            features: features.unwrap_or_default(),
            all_features: all_features.unwrap_or_default(),
            no_default_features: no_default_features.unwrap_or_default(),
            target_selection: match lib {
                Some(true) => Some(TargetSelection::Lib),
                _ => match bin.clone() {
                    Some(BoolOrString::Bool(true)) => Some(TargetSelection::Bin(None)),
                    Some(BoolOrString::String(s)) => Some(TargetSelection::Bin(Some(s))),
                    _ => None,
                },
            },
            toolchain: toolchain.unwrap_or_else(|| DEFAULT_TOOLCHAIN.to_string()),
            target,
            target_dir,
            readme_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum TargetSelection {
    Lib,
    Bin(Option<String>),
}

impl fmt::Display for TargetSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetSelection::Lib => f.write_str("--lib"),
            TargetSelection::Bin(Some(bin)) => write!(f, "--bin {bin}"),
            TargetSelection::Bin(None) => f.write_str("--bin"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    String(String),
}

impl Default for BoolOrString {
    fn default() -> Self {
        BoolOrString::Bool(false)
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Cargo<T: Default> {
    package: Package<T>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Package<T: Default> {
    metadata: Metadata<T>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Metadata<T: Default> {
    insert_docs: T,
}

fn serialize_target_selection<S>(
    value: &Option<TargetSelection>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    struct Helper {
        lib: Option<bool>,
        bin: Option<BoolOrString>,
    }

    match value {
        Some(value) => match value.clone() {
            TargetSelection::Lib => Helper { lib: Some(true), bin: None },
            TargetSelection::Bin(name) => match name {
                Some(name) => Helper { lib: None, bin: Some(BoolOrString::String(name)) },
                None => Helper { lib: None, bin: Some(BoolOrString::Bool(true)) },
            },
        },
        None => Helper { lib: None, bin: None },
    }
    .serialize(serializer)
}

fn metadata_json<T: Default + DeserializeOwned>(json: &serde_json::Value) -> Result<T> {
    let metadata = <Option<Metadata<T>> as Deserialize>::deserialize(json)
        .wrap_err("failed to deserialize metadata")?;
    Ok(metadata.unwrap_or_default().insert_docs)
}

fn metadata_toml<T: Default + DeserializeOwned>(toml: &str) -> Result<T> {
    let cargo = toml::from_str::<Cargo<T>>(toml).wrap_err("failed to deserialize metadata")?;
    Ok(cargo.package.metadata.insert_docs)
}

fn warn_about_unused_fields(fields: HashMap<String, IgnoredAny>, available_fields: &[&[&str]]) {
    let available_fields = available_fields
        .iter()
        .copied()
        .flatten()
        .copied()
        .map(|s| s.replace('_', "-"))
        .collect::<HashSet<_>>();

    let unknown_fields = fields
        .into_keys()
        .filter(|k| !available_fields.contains(&**k))
        .collect::<Vec<String>>()
        .join(", ");

    if !unknown_fields.is_empty() {
        tracing::warn!("metadata.insert-docs contains unknown fields: {unknown_fields}");
    }
}
