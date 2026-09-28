use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{config::is_lib_like, pretty_log::PrettyLog};
use cargo_metadata::{Package, Target};
use color_eyre::eyre::{Context, OptionExt, Result, bail};
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::error_span;

pub struct Options<'a> {
    // metadata
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
    pub quiet_cargo: bool,
    pub no_deps: bool,

    // flags for rustdoc
    pub document_private_items: bool,

    // logging
    pub log: PrettyLog,
}

/// Package must have a `lib` target.
pub fn generate(options: Options) -> Result<PathBuf> {
    let Options {
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
        quiet_cargo,
        log,
        ..
    } = options;

    let mut command = Command::new("cargo");

    if let Some(toolchain) = toolchain {
        command.arg(format!("+{toolchain}"));
    }

    command.arg("rustdoc");

    if is_lib_like(package_target) {
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
    command.arg("-Z").arg("unstable-options");
    command.arg("--output-format").arg("json");
    command.arg("--message-format").arg("json");

    if document_private_items {
        command.arg("--document-private-items");
    }

    if quiet || quiet_cargo {
        command.stderr(Stdio::null());
    }

    if !quiet_cargo {
        command.stderr(Stdio::inherit());

        // the command invocation will write directly to the terminal
        // setting this flag here will make the log insert a newline
        // before the next log message
        log.foreign_write_incoming();
    }

    let output = command.output().wrap_err("failed to spawn cargo")?;

    if !output.status.success() {
        if quiet {
            bail!("you shouldn't be able to see this :/");
        } else {
            if quiet_cargo {
                // write an empty line to separate our messages from the invoked command
                log.foreign_write_incoming();
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }

            bail!("Failed to build rustdoc JSON (see stderr above)");
        }
    }

    let mut artifact = None;

    for message in cargo_metadata::Message::parse_stream(&output.stdout[..]) {
        let cargo_metadata::Message::CompilerArtifact(new_artifact) = message? else {
            continue;
        };

        if new_artifact.package_id != package.id {
            continue;
        }

        if !new_artifact.target.doc {
            continue;
        }

        if artifact.is_none() {
            artifact = Some(new_artifact);
        } else {
            bail!("multiple eligable artifacts? this is a bug");
        }
    }

    let Some(artifact) = artifact else {
        bail!("rustdoc json compiler artifact was not created");
    };

    let path = artifact
        .filenames
        .into_iter()
        .next()
        .map(|p| p.into_std_path_buf())
        .ok_or_eyre("rustdoc json compiler artifact did not produce files")?;

    Ok(path)
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
