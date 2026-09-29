use std::path::{Path, PathBuf};

use anstream::ColorChoice;
use cargo_metadata::{
    Metadata, MetadataCommand, Package, PackageId, PackageName, Target, camino::Utf8PathBuf,
};
use color_eyre::eyre::{OptionExt, Result};
use tracing::info_span;

use crate::{
    config::{self, PackageConfig, is_lib_like},
    util::RelativePath,
};

#[derive(Default)]
pub struct CliContext {
    #[expect(dead_code)]
    pub color: ColorChoice,
    #[expect(dead_code)]
    pub verbose: u8,
    pub quiet: bool,
    pub quiet_cargo: bool,
}

pub struct PackageContext {
    pub id: PackageId,
    pub name: PackageName,
    pub cargo_target: Target,
    pub features: Vec<String>,
    pub manifest_path: Utf8PathBuf,
    pub metadata: Metadata,
    pub readme_path: RelativePath,
    pub target_dir: PathBuf,

    // verbatim from config; forwarded to cargo
    pub toolchain: String,
    pub target: Option<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub no_deps: bool,
    pub document_private_items: bool,

    // verbatim from config; for ourselves
    pub check: bool,
    pub allow_dirty: bool,
    pub allow_staged: bool,
    pub allow_missing_section: bool,
    pub crate_section_name: String,
    pub feature_section_name: String,
    pub feature_into_crate: bool,
    pub crate_into_readme: bool,
    pub hidden_features: Vec<String>,
    pub link_to_latest: bool,
    pub shrink_headings: i8,
    pub feature_label: String,
}

impl PackageContext {
    /// Combines package information and configuration, distilling configuration values,
    /// calling cargo-metadata to get more information.
    ///
    /// If the target selection does not match any target then this returns Ok(None).
    pub fn resolve(pkg: &Package, cfg: &PackageConfig) -> Result<Option<Self>> {
        let target = match &cfg.target_selection {
            Some(target_selection) => match target_selection {
                config::TargetSelection::Lib => {
                    pkg.targets.iter().find(|t| t.doc && is_lib_like(t))
                }
                config::TargetSelection::Bin(bin) => match bin {
                    Some(bin_name) => {
                        pkg.targets.iter().find(|t| t.doc && t.is_bin() && t.name == *bin_name)
                    }
                    None => pkg.targets.iter().find(|t| t.doc && t.is_bin()),
                },
            },
            None => {
                let lib = pkg.targets.iter().find(|t| t.doc && is_lib_like(t));
                let bin = || pkg.targets.iter().find(|t| t.doc && t.is_bin());
                lib.or_else(bin)
            }
        };

        let Some(target) = target else {
            return Ok(None);
        };

        let relative_readme_path = if let Some(path) = cfg.readme_path.as_deref() {
            path
        } else if let Some(path) = pkg.readme.as_deref() {
            path.as_std_path()
        } else {
            Path::new("README.md")
        };

        let readme_path = RelativePath::from_origin_relative(
            pkg.manifest_path.parent().ok_or_eyre("manifest path has no parent dir????")?.as_ref(),
            relative_readme_path,
        );

        let mut cmd = MetadataCommand::new();
        cmd.manifest_path(&pkg.manifest_path);

        if cfg.no_default_features {
            cmd.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
        }

        if cfg.all_features {
            cmd.features(cargo_metadata::CargoOpt::AllFeatures);
        }

        if cfg.features.is_empty() {
            cmd.features(cargo_metadata::CargoOpt::SomeFeatures(cfg.features.clone()));
        }

        let metadata = cmd.exec()?;

        let target_dir = match cfg.target_dir.clone() {
            Some(target_dir) => target_dir,
            None => metadata.target_directory.join("insert-docs").into_std_path_buf(),
        };

        Ok(Some(PackageContext {
            id: pkg.id.clone(),
            name: pkg.name.clone(),
            cargo_target: target.clone(),
            features: cfg
                .features
                .iter()
                .filter(|&f| pkg.features.contains_key(f))
                .cloned()
                .collect(),
            metadata,
            manifest_path: pkg.manifest_path.clone(),
            readme_path: readme_path.clone(),
            target_dir,

            // verbatim from config; forwarded to cargo
            toolchain: cfg.toolchain.clone(),
            target: cfg.target.clone(),
            all_features: cfg.all_features,
            no_default_features: cfg.no_default_features,
            no_deps: cfg.no_deps,
            document_private_items: cfg.document_private_items,

            // verbatim from config; for ourselves
            check: cfg.check,
            allow_dirty: cfg.allow_dirty,
            allow_staged: cfg.allow_staged,
            allow_missing_section: cfg.allow_missing_section,
            crate_section_name: cfg.crate_section_name.clone(),
            feature_section_name: cfg.feature_section_name.clone(),
            feature_into_crate: cfg.feature_into_crate,
            crate_into_readme: cfg.crate_into_readme,
            hidden_features: cfg.hidden_features.clone(),
            link_to_latest: cfg.link_to_latest,
            shrink_headings: cfg.shrink_headings,
            feature_label: cfg.feature_label.clone(),
        }))
    }

    pub fn info_span(&self) -> impl Sized {
        info_span!("", package = self.name.as_str()).entered()
    }
}
