use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use cargo_metadata::{Metadata, Package, Target};
use color_eyre::eyre::{Context, Result, bail};
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::error_span;

pub struct Options<'a> {
    // metadata
    pub metadata: &'a Metadata,
    pub package: &'a Package,
    pub package_target: &'a Target,

    // flags for cargo
    pub toolchain: Option<&'a str>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub features: &'a mut dyn Iterator<Item = &'a str>,
    pub manifest_path: Option<&'a Path>,
    pub target: Option<&'a str>,
    pub target_dir: Option<&'a Path>,
    pub quiet: bool,
    pub no_deps: bool,

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
pub fn generate(options: Options) -> Result<(Output, PathBuf)> {
    let Options {
        metadata,
        package,
        package_target,
        toolchain,
        all_features,
        no_default_features,
        features,
        document_private_items,
        manifest_path,
        target,
        target_dir,
        no_deps,
        quiet,
        output: output_option,
    } = options;

    let mut command = Command::new("cargo");

    if let Some(toolchain) = toolchain {
        command.arg(format!("+{toolchain}"));
    }

    command.arg("rustdoc");

    if package_target.is_lib() {
        command.arg("--lib");
    } else if package_target.is_bin() {
        command.arg("--bin").arg(&package_target.name);
    } else {
        bail!("target must be lib or bin")
    }

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

    if let Some(target_dir) = target_dir {
        command.arg("--target-dir");
        command.arg(target_dir);
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

    if no_deps {
        command.arg("--no-deps");
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

    let output = result.wrap_err_with(|| format!("failed to run {command:?}"))?;

    let mut path = match target_dir {
        Some(path) => path.to_path_buf(),
        None => metadata.target_directory.as_std_path().to_path_buf(),
    };

    path.push("doc");
    path.push(package_target.name.replace('-', "_"));
    path.set_extension("json");

    Ok((output, path))
}

pub fn parse(rustdoc_json: &str, toolchain: &str) -> Result<Crate> {
    #[derive(Deserialize)]
    struct CrateWithJustTheFormatVersion {
        format_version: u32,
    }

    let krate: CrateWithJustTheFormatVersion =
        serde_json::from_str(rustdoc_json).wrap_err("failed to parse generated rustdoc json")?;

    if krate.format_version != rustdoc_types::FORMAT_VERSION {
        let expected = rustdoc_types::FORMAT_VERSION;
        let actual = krate.format_version;

        let _span = error_span!("",
            %toolchain,
            expected = format!("rustdoc json version {expected}"),
            actual = format!("rustdoc json version {actual}"),
        )
        .entered();

        bail!("the chosen rust toolchain is not compatible");
    }

    serde_json::from_str(rustdoc_json).wrap_err("failed to parse generated rustdoc json")
}
