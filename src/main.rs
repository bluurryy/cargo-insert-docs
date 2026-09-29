#![doc = include_str!("../README.md")]
#![allow(
    // ifs are intentionally uncollapsed to make the logic clearer
    clippy::collapsible_if,
    clippy::collapsible_else_if,
)]

mod cli;
mod config;
mod edit_crate_docs;
mod extract_crate_docs;
mod extract_feature_docs;
mod git;
mod markdown;
mod markdown_rs;
mod package_context;
mod pretty_log;
mod rustdoc_json;
mod string_replacer;
#[cfg(test)]
mod tests;

extern crate alloc;

use core::fmt::Write;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use cargo_metadata::{Metadata, MetadataCommand, Package};
use color_eyre::eyre::{OptionExt, Result, WrapErr as _, bail, eyre};
use mimalloc::MiMalloc;
use relative_path::PathExt;
use serde::Serialize;
use tracing::{Level, error_span, info_span, trace};

use pretty_log::{PrettyLog, WithResultSeverity as _};

use crate::{
    cli::Cli,
    config::{PackageConfig, PackageConfigPatch, WorkspaceConfig, WorkspaceConfigPatch},
    package_context::{CliContext, PackageContext},
    pretty_log::AnyWrite,
    string_replacer::StringReplacer,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.cfg.print_supported_toolchain {
        println!("{}", config::DEFAULT_TOOLCHAIN);
        return ExitCode::SUCCESS;
    }

    let stream: Box<dyn AnyWrite> = if cli.cfg.quiet {
        Box::new(io::empty())
    } else {
        Box::new(anstream::AutoStream::new(std::io::stderr(), cli.cfg.color))
    };

    let log = PrettyLog::new(stream);
    log.source_info(cli.cfg.verbose >= 2);

    let log_level = if cli.cfg.verbose >= 1 { "trace" } else { "info" };
    log.install(&format!("cargo_insert_docs={log_level}"));

    if let Err(err) = try_main(&cli, &log) {
        log.print_report(&err);
    }

    log.print_tally();

    if log.tally().errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn try_main(cli: &Cli, log: &PrettyLog) -> Result<()> {
    let mut cmd = MetadataCommand::new();

    if let Some(manifest_path) = cli.cfg.manifest_path.as_deref() {
        cmd.manifest_path(manifest_path);
    }

    let metadata = cmd.exec()?;
    let (workspace_workspace_config_patch, workspace_package_config_patch) =
        config::read_workspace_config_patch(&metadata.workspace_metadata)?;

    let workspace_cfg =
        workspace_workspace_config_patch.apply(&cli.workspace_config_patch).finish();

    let mut packages: Vec<&Package> = if workspace_cfg.workspace {
        metadata.workspace_members.iter().map(|p| &metadata[p]).collect()
    } else if workspace_cfg.package.is_empty() {
        assert!(
            metadata.workspace_default_members.is_available(),
            "to infer the current package, cargo of rust version 1.71 or higher is required"
        );

        if metadata.workspace_default_members.is_available() {
            (*metadata.workspace_default_members).iter().map(|p| &metadata[p]).collect()
        } else {
            bail!("`cargo-insert-docs` requires a cargo version >= 1.71");
        }
    } else {
        find_packages_by_name(&metadata, &workspace_cfg.package)?
    };

    let excluded_packages = workspace_cfg
        .exclude
        .iter()
        .map(|name| find_package_by_name(&metadata, name))
        .collect::<Result<HashSet<_>>>()?;

    packages.retain(|id| !excluded_packages.contains(id));

    if packages.is_empty() {
        bail!("no packages selected");
    }

    // Error if a feature is not available in any selected package.
    // This is an error solely because this is likely a user bug.
    if !cli.cfg.print_config {
        let pkg = workspace_package_config_patch.clone().apply(&cli.package_config_patch).finish();

        let all_available_features = packages
            .iter()
            .flat_map(|p| p.features.keys())
            .map(|s| s.as_str())
            .collect::<HashSet<&str>>();

        let unavailable_features = pkg
            .features
            .iter()
            .map(|s| s.as_str())
            .filter(|f| !all_available_features.contains(f))
            .collect::<Vec<&str>>();

        if !unavailable_features.is_empty() {
            let contain_these_features = if unavailable_features.len() == 1 {
                "contains this feature"
            } else {
                "contain these features"
            };

            let unavailable_features = unavailable_features.join(", ");

            bail!("none of the selected packages {contain_these_features}: {unavailable_features}");
        }
    }

    struct PackageConfigAndPatch {
        config: PackageConfig,
        patch: PackageConfigPatch,
    }

    // Collect package all package configurations before doing anything else,
    // so we error early if there is a configuration error.
    //
    // We also need this data for `--print-config`
    let package_config_and_patches = packages
        .iter()
        .map(|package| {
            let _span = info_span!("", package = package.name.as_str()).entered();

            let toml = RelativePath::relative_to_parent(package.manifest_path.as_ref())?
                .read_to_string()?;
            let patch = config::read_package_config_patch(&toml)?;

            let combined_patch =
                workspace_package_config_patch.apply(&patch).apply(&cli.package_config_patch);

            if combined_patch.bin.is_some() && combined_patch.lib.is_some() {
                bail!("`lib` and `bin` are both set, you have to choose one or the other");
            }

            let config = combined_patch.finish();

            Ok(PackageConfigAndPatch { config, patch })
        })
        .collect::<Result<Vec<_>>>()?;

    if cli.cfg.print_config {
        #[derive(Serialize)]
        struct WorkspaceAndPackageConfigPatch<'a> {
            #[serde(flatten)]
            workspace: &'a WorkspaceConfigPatch,
            #[serde(flatten)]
            package: &'a PackageConfigPatch,
        }

        #[derive(Serialize)]
        struct WorkspaceAndPackageConfig<'a> {
            #[serde(flatten)]
            workspace: &'a WorkspaceConfig,
            #[serde(flatten)]
            package: &'a PackageConfig,
        }

        #[derive(Serialize)]
        struct PerPackage<'a> {
            package: HashMap<&'a str, &'a PackageConfigPatch>,
            resolved: HashMap<&'a str, WorkspaceAndPackageConfig<'a>>,
        }

        #[derive(Serialize)]
        struct Table<'a> {
            cli: WorkspaceAndPackageConfigPatch<'a>,
            workspace: WorkspaceAndPackageConfigPatch<'a>,
        }

        let mut out = toml::to_string(&Table {
            cli: WorkspaceAndPackageConfigPatch {
                workspace: &cli.workspace_config_patch,
                package: &cli.package_config_patch,
            },
            workspace: WorkspaceAndPackageConfigPatch {
                workspace: &workspace_workspace_config_patch,
                package: &workspace_package_config_patch,
            },
        })
        .wrap_err("toml serialization failed")?;

        for (package, PackageConfigAndPatch { config, patch }) in
            packages.iter().zip(package_config_and_patches)
        {
            let _span = info_span!("", package = package.name.as_str()).entered();
            let name = package.name.as_str();

            out.push('\n');

            out.push_str(
                &toml::to_string(&PerPackage {
                    package: HashMap::from_iter([(name, &patch)]),
                    resolved: HashMap::from_iter([(
                        name,
                        WorkspaceAndPackageConfig { workspace: &workspace_cfg, package: &config },
                    )]),
                })
                .wrap_err("toml serialization failed")?,
            );
        }

        log.foreign_write_incoming();
        println!("{out}");
        return Ok(());
    }

    // We first resolve all the package configs.
    // This way we error early if there are configuration errors.
    let pkgs = packages
        .iter()
        .zip(&package_config_and_patches)
        .map(|(pkg, PackageConfigAndPatch { config: cfg, .. })| {
            let _span = info_span!("", package = pkg.name.as_str()).entered();
            PackageContext::resolve(pkg, cfg)
        })
        .filter_map(|r| r.transpose())
        .collect::<Result<Vec<_>>>()?;

    if pkgs.is_empty() {
        let _span = workspace_package_config_patch
            .finish()
            .target_selection
            .map(|filter| error_span!("", %filter).entered());

        bail!("no target found to document");
    }

    check_version_control(&pkgs)?;

    // If we're not dealing with a workspace and an explicitly set package then
    // we don't log the package name, since it doesn't provide useful information.
    let log_package_name = workspace_cfg.workspace || workspace_cfg.package.is_empty();

    for pkg in &pkgs {
        let log_package_name =
            log_package_name || (*pkg.metadata.workspace_default_members).len() > 1;

        let _span = log_package_name.then(|| pkg.info_span());
        run_package(
            log,
            &CliContext {
                color: cli.cfg.color,
                verbose: cli.cfg.verbose,
                quiet: cli.cfg.quiet,
                quiet_cargo: cli.cfg.quiet_cargo,
            },
            pkg,
        );
    }

    Ok(())
}

// Modified from `fn check_version_control` in `rust-lang/cargo/src/cargo/ops/fix/mod.rs`.
fn check_version_control(pkgs: &[PackageContext]) -> Result<()> {
    if pkgs.is_empty() {
        return Ok(());
    }

    // bool: allow_staged
    let mut files: Vec<(&Path, bool)> = vec![];

    for pkg in pkgs {
        if pkg.check || pkg.allow_dirty {
            continue;
        }

        if pkg.feature_into_crate {
            let path = pkg.cargo_target.src_path.as_std_path();
            files.push((path, pkg.allow_staged));
        }

        if pkg.crate_into_readme {
            let path = pkg.readme_path.full_path.as_path();
            files.push((path, pkg.allow_staged));
        }
    }

    let status = git::file_status(files.iter().map(|f| f.0));

    let error_files = files
        .iter()
        .zip(status.iter())
        .filter_map(|((path, _), status)| match status {
            git::Status::Error(error) => Some((path, error)),
            _ => None,
        })
        .collect::<Vec<_>>();

    let dirty_files = files
        .iter()
        .zip(status.iter())
        .filter_map(|((path, _), status)| match status {
            git::Status::Dirty => Some(path),
            _ => None,
        })
        .collect::<Vec<_>>();

    let staged_files = files
        .iter()
        .zip(status.iter())
        .filter_map(|((path, allow_staged), status)| match status {
            git::Status::Staged if !allow_staged => Some(path),
            _ => None,
        })
        .collect::<Vec<_>>();

    if error_files.is_empty() && dirty_files.is_empty() && staged_files.is_empty() {
        return Ok(());
    }

    let display_path = |path: &Path| -> String {
        path.relative_to(pkgs[0].metadata.workspace_root.as_std_path())
            .map(|p| p.to_string())
            .unwrap_or_else(|_| path.display().to_string())
    };

    let mut files_list = String::new();

    for (path, error) in error_files {
        let path = display_path(path);
        _ = files_list.write_fmt(format_args!("  * {path} (error: {error})\n"));
    }

    for path in dirty_files {
        let path = display_path(path);
        _ = files_list.write_fmt(format_args!("  * {path} (dirty)\n"));
    }

    for path in staged_files {
        let path = display_path(path);
        _ = files_list.write_fmt(format_args!("  * {path} (staged)\n"));
    }

    bail!(
        "the working directory of this package has uncommitted changes, and \n\
            `cargo insert-docs` can potentially perform destructive changes;\n\
            if you'd like to suppress this error pass `--allow-dirty`, \n\
            or commit the changes to these files:\n\
            \n\
            {files_list}\n\
         "
    );
}

fn run_package(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) {
    if pkg.feature_into_crate {
        task(
            log,
            cli,
            pkg,
            "feature documentation",
            "crate documentation",
            insert_features_into_docs,
        );
    }

    if pkg.crate_into_readme {
        task(log, cli, pkg, "crate documentation", "readme", insert_docs_into_readme);
    }
}

fn find_packages_by_name(
    metadata: &Metadata,
    package_names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Vec<&Package>> {
    package_names.into_iter().map(|name| find_package_by_name(metadata, name.as_ref())).collect()
}

fn find_package_by_name<'a>(metadata: &'a Metadata, package_name: &str) -> Result<&'a Package> {
    for workspace_member in &metadata.workspace_members {
        let package = &metadata[workspace_member];

        if package.name.as_str() == package_name {
            return Ok(package);
        }
    }

    bail!("no package named \"{package_name}\" found")
}

// for better error messages when reading / writing files
#[derive(Clone)]
struct RelativePath {
    full_path: PathBuf,
    relative: PathBuf,
}

impl RelativePath {
    fn from_origin_relative(origin: &Path, relative: &Path) -> Self {
        Self { full_path: origin.join(relative), relative: relative.into() }
    }

    fn relative_to_parent(path: &Path) -> Result<Self> {
        Ok(Self {
            relative: path.file_name().ok_or_eyre("path has no file name")?.into(),
            full_path: path.into(),
        })
    }

    fn read_to_string(&self) -> Result<String> {
        let _span = error_span!("", path = %self.full_path.display()).entered();

        fs::read_to_string(&self.full_path)
            .with_context(|| format!("failed to read {}", self.relative.display()))
    }

    fn write(&self, contents: &str) -> Result<()> {
        let _span = error_span!("", path = %self.full_path.display()).entered();

        fs::write(&self.full_path, contents)
            .with_context(|| format!("failed to write {}", self.relative.display()))
    }
}

fn task(
    log: &PrettyLog,
    cli: &CliContext,
    pkg: &PackageContext,
    from: &str,
    to: &str,
    f: fn(&PrettyLog, &CliContext, &PackageContext) -> Result<()>,
) {
    let task_name = if pkg.check {
        format!("checking {from} in {to}")
    } else {
        format!("insert {from} into {to}")
    };

    let _span = info_span!("", task = task_name).entered();

    trace!("starting task");

    let start = Instant::now();

    if let Err(report) = f(log, cli, pkg) {
        let context = if pkg.check {
            format!("checking {from} failed")
        } else {
            format!("could not {task_name}")
        };

        log.print_report(&report.wrap_err(context));
    }

    trace!("finished in {:?}", start.elapsed());
}

fn insert_features_into_docs(
    _log: &PrettyLog,
    _cli: &CliContext,
    pkg: &PackageContext,
) -> Result<()> {
    let not_found_level = if pkg.allow_missing_section { Level::WARN } else { Level::ERROR };

    let target_path = pkg.cargo_target.src_path.as_std_path();
    let target_src = read_to_string(target_path)?;

    let Some(feature_docs_section) =
        edit_crate_docs::FeatureDocsSection::find(&target_src, &pkg.feature_section_name)?
    else {
        let target_name = target_path
            .file_name()
            .map(|n| Path::new(n).display().to_string())
            .unwrap_or_else(|| "crate docs".into());

        let _span = info_span!("",
            path = %target_path.display(),
            section_name = pkg.feature_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {target_name}")).with_severity(not_found_level);
    };

    let cargo_toml =
        RelativePath::relative_to_parent(pkg.manifest_path.as_std_path())?.read_to_string()?;

    let hidden_features = pkg.hidden_features.iter().map(|s| s.as_str()).collect::<HashSet<&str>>();

    let feature_docs =
        extract_feature_docs::extract(&cargo_toml, &pkg.feature_label, &hidden_features)
            .wrap_err("failed to parse Cargo.toml")?;

    let new_target_src = feature_docs_section.replace(&feature_docs)?;

    if new_target_src != target_src {
        if pkg.check {
            bail!("feature documentation is stale");
        }

        write(target_path, new_target_src.as_bytes())?;
    }

    Ok(())
}

fn insert_docs_into_readme(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<()> {
    let not_found_level = if pkg.allow_missing_section { Level::WARN } else { Level::ERROR };

    let readme_path = &pkg.readme_path;
    let readme = readme_path.read_to_string().with_severity(not_found_level)?;

    let section_name = &pkg.crate_section_name;
    let subsections = markdown::find_subsections(&readme, section_name)?;

    let new_readme = if !subsections.is_empty() {
        let crate_docs = extract_crate_docs::extract(log, cli, pkg)?;
        let [without_definitions, definitions] = markdown::extract_definitions(&crate_docs);

        let mut new_readme = StringReplacer::new(&readme);
        let last_subsection_i = subsections.len().saturating_sub(1);

        for (i, (section, name)) in subsections.into_iter().enumerate() {
            let replace_with_section = markdown::find_section(&without_definitions, &format!("{section_name} {name}")).ok_or_else(|| eyre!("\"{section_name}\" subsection \"{name}\" is contained in readme but missing from crate docs"))?;

            if i == last_subsection_i {
                let replace_with = &without_definitions[replace_with_section.content_span];
                new_readme
                    .insert(section.span.start, format!("<!-- {section_name} {name} start -->"));
                new_readme.replace(section.span.clone(), replace_with);
                new_readme.insert(section.span.end, "\n");
                new_readme.insert(section.span.end, &definitions);
                new_readme.insert(section.span.end, format!("<!-- {section_name} {name} end -->"));
            } else {
                let replace_with = &without_definitions[replace_with_section.span];
                new_readme.replace(section.span.clone(), replace_with);
            }
        }

        new_readme.finish()
    } else if let Some(section) = markdown::find_section(&readme, &pkg.crate_section_name) {
        let crate_docs = extract_crate_docs::extract(log, cli, pkg)?;
        let mut new_readme = readme.clone();
        new_readme.replace_range(section.content_span, &format!("\n{crate_docs}\n"));
        new_readme
    } else {
        let _span = info_span!("",
            path = %readme_path.full_path.display(),
            section_name = pkg.crate_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {}", readme_path.relative.display()))
            .with_severity(not_found_level);
    };

    if readme != new_readme {
        if pkg.check {
            bail!("crate documentation is stale");
        }

        readme_path.write(&new_readme)?;
    }

    Ok(())
}

/// Better error reporting version of [`std::fs::read_to_string`].
fn read_to_string(path: &Path) -> Result<String> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    fs::read_to_string(path).with_context(|| format!("failed to read {file_name}"))
}

/// Better error reporting version of [`std::fs::write`].
fn write(path: &Path, content: &[u8]) -> Result<()> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    fs::write(path, content).with_context(|| format!("failed to write to {file_name}"))
}
