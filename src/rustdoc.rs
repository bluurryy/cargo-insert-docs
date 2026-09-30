mod cargo_rustdoc;
mod unit_graph;

use std::path::PathBuf;

use crate::{PackageContext, package_context::CliContext, pretty_log::PrettyLog, util};
use cargo_metadata::Target;
use color_eyre::eyre::{Context, OptionExt, Result, bail};
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::error_span;

pub fn generate(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<Crate> {
    create_doc_directory(log, cli, pkg)?;
    let path = generate_and_get_path(log, cli, pkg)?;
    let json = util::read_to_string(&path)?;
    let krate = parse(&json, &pkg.toolchain)?;
    Ok(krate)
}

fn generate_and_get_path(
    log: &PrettyLog,
    cli: &CliContext,
    pkg: &PackageContext,
) -> Result<PathBuf> {
    let stdout = cargo_rustdoc::command().run(log, cli, pkg)?;
    let mut json_files = Vec::new();

    for message in cargo_metadata::Message::parse_stream(&stdout[..]) {
        if let cargo_metadata::Message::CompilerArtifact(artifact) = message?
            && artifact.package_id == pkg.id
            && same_package_target(&artifact.target, &pkg.cargo_target)
        {
            json_files.extend(
                artifact.filenames.into_iter().filter(|path| path.extension() == Some("json")),
            );
        }
    }

    if json_files.len() > 1 {
        bail!(
            "rustdoc produced multiple json artifacts:\n{}",
            json_files.iter().map(|p| p.as_str()).collect::<Vec<_>>().join("\n"),
        );
    }

    let Some(json_file) = json_files.into_iter().next() else {
        bail!("rustdoc did not produce a json artifact");
    };

    Ok(json_file.into_std_path_buf())
}

/// When using a custom build-dir, cargo may reuse the cached rustdoc json file and copy it to
/// `doc/` in the target directory. Currently in such cases the `doc/` directory may not have
/// been created and cargo errors with (paraphrased):
///
/// ```txt
/// error: failed to link or copy `$BUILD_DIR/**/out/$PACKAGE.json` to `$TARGET_DIR/**/doc/$PACKAGE.json`
/// ```
///
/// So we create that directory ourselves for now. We should find or open an issue about this in cargo.
fn create_doc_directory(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result {
    let doc = doc_directory(log, cli, pkg)?;

    let _span = error_span!("", path = %doc.display()).entered();
    std::fs::create_dir_all(doc).wrap_err("failed to create rustdoc output directory")?;

    Ok(())
}

/// Constructs the assumed path of the `doc` directory using the build graph.
fn doc_directory(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<PathBuf> {
    let stdout = cargo_rustdoc::command().cargo_arg("--unit-graph").run(log, cli, pkg)?;
    let graph = unit_graph::parse(&stdout)?;

    let matching_units = graph
        .roots
        .iter()
        .filter_map(|&root| graph.units.get(root))
        .filter(|unit| {
            unit.mode == "doc"
                && unit.pkg_id == pkg.id
                && same_package_target(&unit.target, &pkg.cargo_target)
        })
        .collect::<Vec<_>>();

    if matching_units.len() > 1 {
        let _span = error_span!(
            "",
            targets = matching_units
                .iter()
                .map(|u| u.target.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .entered();

        bail!("multiple build targets, choose one with `--target`");
    }

    let unit = matching_units.into_iter().next().ok_or_eyre("no build target found")?;

    let mut path = pkg.target_dir.clone();

    if let Some(platform) = unit.platform.as_deref() {
        path.push(platform);
    }

    path.push("doc");

    Ok(path)
}

fn same_package_target(a: &Target, b: &Target) -> bool {
    a.name == b.name && a.kind == b.kind && a.src_path == b.src_path
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
