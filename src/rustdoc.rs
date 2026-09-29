use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    PackageContext, config::is_lib_like, package_context::CliContext, pretty_log::PrettyLog, util,
};
use anstream::ColorChoice;
use color_eyre::eyre::{Context, OptionExt, Result, bail};
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::error_span;

pub fn generate(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<Crate> {
    let path = generate_and_get_path(log, cli, pkg)?;
    let json = util::read_to_string(&path)?;
    let krate = parse(&json, &pkg.toolchain)?;
    Ok(krate)
}

/// Package must have a `lib` target.
fn generate_and_get_path(
    log: &PrettyLog,
    cli: &CliContext,
    pkg: &PackageContext,
) -> Result<PathBuf> {
    let mut cmd = Command::new("cargo");

    cmd.arg(format!("+{}", pkg.toolchain));

    cmd.args([
        "rustdoc",
        "-Z",
        "unstable-options",
        "--output-format",
        "json",
        "--message-format",
        "json-render-diagnostics",
    ]);

    if is_lib_like(&pkg.cargo_target) {
        cmd.arg("--lib");
    } else if pkg.cargo_target.is_bin() {
        cmd.arg("--bin").arg(&pkg.cargo_target.name);
    } else {
        bail!("target must be lib or bin")
    }

    if cli.quiet {
        cmd.arg("--quiet");
    }

    cmd.arg("--color").arg(match cli.color {
        ColorChoice::Auto => "auto",
        ColorChoice::Always => "always",
        ColorChoice::Never | ColorChoice::AlwaysAnsi => "never",
    });

    cmd.arg("--manifest-path");
    cmd.arg(&pkg.manifest_path);

    if let Some(target) = pkg.target.as_deref() {
        cmd.arg("--target");
        cmd.arg(target);
    }

    cmd.arg("--target-dir");
    cmd.arg(&pkg.target_dir);

    if pkg.all_features {
        cmd.arg("--all-features");
    }

    if pkg.no_default_features {
        cmd.arg("--no-default-features");
    }

    for feature in &pkg.features {
        cmd.arg("--features").arg(feature);
    }

    cmd.arg("--package").arg(&pkg.id.repr);

    // The rest are flags only for rustdoc
    cmd.arg("--");

    if pkg.document_private_items {
        cmd.arg("--document-private-items");
    }

    if cli.quiet_cargo {
        cmd.stderr(Stdio::null());
    } else {
        log.foreign_write_incoming();
        cmd.stderr(Stdio::inherit());
    }

    let output = cmd.output().wrap_err("failed to spawn cargo")?;

    if !output.status.success() {
        if cli.quiet {
            bail!("you shouldn't be able to see this :/");
        } else {
            bail!("Failed to build rustdoc JSON (see stderr above)");
        }
    }

    let mut eligible_artifacts = Vec::new();

    for message in cargo_metadata::Message::parse_stream(&output.stdout[..]) {
        let cargo_metadata::Message::CompilerArtifact(artifact) = message? else {
            continue;
        };

        if artifact.package_id != pkg.id {
            continue;
        }

        if artifact.target != pkg.cargo_target {
            continue;
        }

        eligible_artifacts.push(artifact);
    }

    if eligible_artifacts.len() > 1 {
        bail!(
            "rustdoc json produced multiple compiler artifacts; there are probably multiple build targets configured; choose one with `--target`"
        );
    }

    let Some(artifact) = eligible_artifacts.into_iter().next() else {
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

fn parse(rustdoc_json: &str, toolchain: &str) -> Result<Crate> {
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
