use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

use camino::Utf8PathBuf;
use cargo_metadata::{Metadata, PackageId};
use color_eyre::eyre::{Context, OptionExt, Result, bail, ensure};
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::error_span;

pub struct Options<'a> {
    // flags for cargo
    pub toolchain: Option<&'a str>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub features: &'a mut dyn Iterator<Item = &'a str>,
    pub manifest_path: Option<&'a Path>,
    pub target: Option<&'a str>,
    pub quiet: bool,

    // flags for rustdoc
    pub document_private_items: bool,

    // process handling
    pub output: CommandOutput,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CommandOutput {
    Inherit,
    Ignore,
    Collect,
}

/// Package must have a `lib` target.
pub fn generate(metadata: &Metadata, package_id: &PackageId, options: Options) -> Result<Output> {
    let package =
        metadata.packages.iter().find(|p| &p.id == package_id).ok_or_eyre("invalid package id")?;

    ensure!(package.targets.iter().any(|t| t.is_lib()), "package has no lib target");

    let Options {
        toolchain,
        all_features,
        no_default_features,
        features,
        document_private_items,
        manifest_path,
        target,
        quiet,
        output: output_option,
    } = options;

    let mut command = Command::new("cargo");

    if let Some(toolchain) = toolchain {
        command.arg(format!("+{toolchain}"));
    }

    command.arg("rustdoc");
    command.arg("--lib");

    if quiet {
        command.arg("--quiet");
    }

    command.arg("--color").arg("always");

    if let Some(manifest_path) = manifest_path {
        command.arg("--manifest-path");
        command.arg(manifest_path);
    }

    if let Some(target) = target {
        command.arg("--target");
        command.arg(target);
    }

    if all_features {
        command.arg("--all-features");
    }

    if no_default_features {
        command.arg("--no-default-features");
    }

    for feature in features {
        command.arg("--features").arg(feature);
    }

    command.arg("--package").arg(&package.id.repr);
    command.arg("--");
    command.arg("-Z").arg("unstable-options");
    command.arg("--output-format").arg("json");

    if document_private_items {
        command.arg("--document-private-items");
    }

    if matches!(output_option, CommandOutput::Ignore) {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }

    let result = if matches!(output_option, CommandOutput::Collect) {
        command.output()
    } else {
        command.status().map(|status| Output { status, stdout: vec![], stderr: vec![] })
    };

    result.wrap_err_with(|| format!("failed to run {command:?}"))
}

pub fn path(metadata: &Metadata, package_id: &PackageId) -> Result<Utf8PathBuf> {
    let lib = metadata[package_id]
        .targets
        .iter()
        .find(|t| t.is_lib())
        .ok_or_eyre("package has no lib target")?;

    let mut path = metadata.target_directory.clone();
    path.push("doc");
    path.push(&lib.name);
    path.set_extension("json");
    Ok(path)
}

pub fn parse(rustdoc_json: &str) -> Result<Crate> {
    #[derive(Deserialize)]
    struct CrateWithJustTheFormatVersion {
        format_version: u32,
    }

    let krate: CrateWithJustTheFormatVersion =
        serde_json::from_str(rustdoc_json).wrap_err("failed to parse generated rustdoc json")?;

    if krate.format_version != rustdoc_types::FORMAT_VERSION {
        let expected = rustdoc_types::FORMAT_VERSION;
        let actual = krate.format_version;

        let help = if actual > expected {
            "update `cargo-insert-docs` or use `--toolchain nightly-2025-07-16`"
        } else {
            "upgrade your nightly toolchain"
        };

        let _span = error_span!("", %help).entered();

        bail!(
            "`cargo-insert-docs` requires rustdoc json format version {expected} \
            but rustdoc produced version {actual}"
        );
    }

    serde_json::from_str(rustdoc_json).wrap_err("failed to parse generated rustdoc json")
}
