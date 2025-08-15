use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
};

use color_eyre::eyre::{Result, WrapErr as _};
use macro_rules_attribute::derive;
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, IgnoredAny},
};

use crate::{Args, ColorChoice, Command};

pub const DEFAULT_FEATURE_LABEL: &str = "**`{feature}`**";
pub const DEFAULT_FEATURE_SECTION_NAME: &str = "feature documentation";
pub const DEFAULT_CRATE_SECTION_NAME: &str = "crate documentation";
pub const DEFAULT_TOOLCHAIN: &str = "nightly-2025-08-02";

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

pub struct ArgsConfig {
    pub cli: CliConfig,
    pub workspace_patch: WorkspaceConfigPatch,
    pub package_patch: PackageConfigPatch,
}

impl ArgsConfig {
    pub fn from_args(args: &Args) -> Self {
        let cli = CliConfig::from_args(args);
        let workspace_patch = WorkspaceConfigPatch::from_args(args);
        let package_patch = PackageConfigPatch::from_args(args);

        Self { cli, workspace_patch, package_patch }
    }
}

pub struct CliConfig {
    pub print_supported_toolchain: bool,
    pub print_config: bool,
    pub color: ColorChoice,
    pub verbose: bool,
    pub quiet: bool,
    pub quiet_cargo: bool,
    pub workspace: bool,
    pub manifest_path: Option<PathBuf>,
}

impl CliConfig {
    pub fn from_args(args: &Args) -> Self {
        let Args {
            print_supported_toolchain,
            color,
            verbose,
            quiet,
            quiet_cargo,
            workspace,
            ref manifest_path,
            print_config,
            ..
        } = *args;

        Self {
            print_supported_toolchain,
            print_config,
            color: color.unwrap_or(ColorChoice::Auto),
            verbose,
            quiet,
            quiet_cargo: quiet || quiet_cargo,
            workspace,
            manifest_path: manifest_path.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct WorkspaceConfig {
    pub package: Vec<String>,
    pub exclude: Vec<String>,
}

pub fn read_workspace_config(
    json: &serde_json::Value,
) -> Result<(WorkspaceConfigPatch, PackageConfigPatch)> {
    let wrk: WorkspaceConfigPatch = metadata_json(json)?;
    let pkg: PackageConfigPatch = metadata_json(json)?;
    let fields: HashMap<String, IgnoredAny> = metadata_json(json)?;
    warn_about_unused_fields(
        fields,
        WorkspaceConfigPatch::FIELDS
            .iter()
            .copied()
            .chain(PackageConfigPatch::FIELDS.iter().copied())
            .collect(),
    );
    Ok((wrk, pkg))
}

pub fn read_package_config(toml: &str) -> Result<PackageConfigPatch> {
    let pkg: PackageConfigPatch = metadata_toml(toml)?;
    let fields: HashMap<String, IgnoredAny> = metadata_toml(toml)?;
    warn_about_unused_fields(fields, PackageConfigPatch::FIELDS.iter().copied().collect());
    Ok(pkg)
}

#[derive(Default, Clone, Deserialize, Serialize, Fields!)]
#[serde(default, rename_all = "kebab-case")]
pub struct WorkspaceConfigPatch {
    package: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

impl WorkspaceConfigPatch {
    pub fn from_args(args: &Args) -> Self {
        let Args { package, exclude, .. } = args;

        Self {
            package: (!package.is_empty()).then(|| package.clone()),
            exclude: (!exclude.is_empty()).then(|| exclude.clone()),
        }
    }

    pub fn apply(&self, overwrite: &Self) -> Self {
        let mut this = self.clone();

        if let Some(package) = &overwrite.package {
            this.package.get_or_insert_with(Vec::new).extend(package.clone());
        }
        if let Some(exclude) = &overwrite.exclude {
            this.exclude.get_or_insert_with(Vec::new).extend(exclude.clone());
        }

        this
    }

    pub fn finish(self) -> WorkspaceConfig {
        let Self { package, exclude } = self;
        WorkspaceConfig {
            package: package.unwrap_or_default(),
            exclude: exclude.unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
pub struct PackageConfig {
    pub feature_into_crate: bool,
    pub crate_into_readme: bool,
    pub feature_label: String,
    pub feature_section_name: String,
    pub crate_section_name: String,
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
    pub target_selection: Option<TargetSelection>,
    pub toolchain: String,
    pub target: Option<String>,
    pub target_dir: Option<PathBuf>,
    pub readme_path: Option<PathBuf>,
}

#[derive(Default, Clone, Deserialize, Serialize, Fields!)]
#[serde(default, rename_all = "kebab-case")]
pub struct PackageConfigPatch {
    pub feature_into_crate: Option<bool>,
    pub crate_into_readme: Option<bool>,
    pub feature_label: Option<String>,
    pub feature_section_name: Option<String>,
    pub crate_section_name: Option<String>,
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
    pub target_selection: Option<TargetSelection>,
    pub toolchain: Option<String>,
    pub target: Option<String>,
    pub target_dir: Option<PathBuf>,
    pub readme_path: Option<PathBuf>,
}

impl PackageConfigPatch {
    pub fn from_args(args: &Args) -> Self {
        let Args {
            command,
            feature_label,
            feature_section_name,
            crate_section_name,
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
            target_selection,
            toolchain,
            target,
            target_dir,
            readme_path,
            ..
        } = args;

        Self {
            feature_into_crate: command.map(|c| c == Command::FeatureIntoCrate),
            crate_into_readme: command.map(|c| c == Command::CrateIntoReadme),
            feature_label: feature_label.clone(),
            feature_section_name: feature_section_name.clone(),
            crate_section_name: crate_section_name.clone(),
            link_to_latest: link_to_latest.then_some(true),
            document_private_items: document_private_items.then_some(true),
            no_deps: no_deps.then_some(true),
            check: check.then_some(true),
            allow_missing_section: allow_missing_section.then_some(true),
            allow_dirty: allow_dirty.then_some(true),
            allow_staged: allow_staged.then_some(true),
            features: (!features.is_empty()).then(|| {
                // features are already comma separated, we still need to make them space separated
                features.iter().flat_map(|f| f.split(' ').map(|s| s.to_string())).collect()
            }),
            all_features: all_features.then_some(true),
            no_default_features: no_default_features.then_some(true),
            target_selection: if target_selection.lib {
                Some(TargetSelection::Lib)
            } else {
                target_selection.bin.clone().map(TargetSelection::Bin)
            },
            toolchain: toolchain.clone(),
            target: target.clone(),
            target_dir: target_dir.clone(),
            readme_path: readme_path.clone(),
        }
    }

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
            this.features.get_or_insert_with(Vec::new).extend(features.clone());
        }
        if let Some(all_features) = overwrite.all_features {
            this.all_features = Some(all_features);
        }
        if let Some(no_default_features) = overwrite.no_default_features {
            this.no_default_features = Some(no_default_features);
        }
        if let Some(target_selection) = &overwrite.target_selection {
            this.target_selection = Some(target_selection.clone());
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
            target_selection,
            toolchain,
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
            target_selection,
            toolchain: toolchain.unwrap_or_else(|| DEFAULT_TOOLCHAIN.to_string()),
            target,
            target_dir,
            readme_path,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
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

fn metadata_json<T: Default + DeserializeOwned>(json: &serde_json::Value) -> Result<T> {
    let metadata = <Metadata<T> as Deserialize>::deserialize(json)
        .wrap_err("failed to deserialize metadata")?;
    Ok(metadata.insert_docs)
}

fn metadata_toml<T: Default + DeserializeOwned>(toml: &str) -> Result<T> {
    let cargo = toml::from_str::<Cargo<T>>(toml).wrap_err("failed to deserialize metadata")?;
    Ok(cargo.package.metadata.insert_docs)
}

fn warn_about_unused_fields(fields: HashMap<String, IgnoredAny>, available_fields: HashSet<&str>) {
    let unused_fields = fields
        .into_keys()
        .filter(|k| available_fields.contains(&**k))
        .collect::<Vec<String>>()
        .join(", ");

    if !unused_fields.is_empty() {
        tracing::warn!("metadata.insert-docs contains unknown fields: {unused_fields}");
    }
}
