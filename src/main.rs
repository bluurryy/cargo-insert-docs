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

use cargo_metadata::{Metadata, MetadataCommand, Package, Target};
use color_eyre::eyre::{OptionExt, Result, WrapErr as _, bail, eyre};
use mimalloc::MiMalloc;
use relative_path::PathExt;
use serde::Serialize;
use tracing::{Level, error_span, info_span, trace};

use pretty_log::{PrettyLog, WithResultSeverity as _};

use crate::{
    cli::Cli,
    config::{
        PackageConfig, PackageConfigPatch, WorkspaceConfig, WorkspaceConfigPatch, is_lib_like,
    },
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
        config::read_workspace_config(&metadata.workspace_metadata)?;

    let workspace = workspace_workspace_config_patch.apply(&cli.workspace_patch).finish();

    let mut packages: Vec<&Package> = if workspace.workspace {
        metadata.workspace_members.iter().map(|p| &metadata[p]).collect()
    } else if workspace.package.is_empty() {
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
        find_packages_by_name(&metadata, &workspace.package)?
    };

    let excluded_packages = workspace
        .exclude
        .iter()
        .map(|name| find_package_by_name(&metadata, name))
        .collect::<Result<HashSet<_>, _>>()?;

    packages.retain(|id| !excluded_packages.contains(id));

    if packages.is_empty() {
        bail!("no packages selected");
    }

    // error if a feature is not available in any selected package
    if !cli.cfg.print_config {
        let pkg = workspace_package_config_patch.clone().apply(&cli.package_patch).finish();

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

    // We first prepare all the contexts for each package.
    // This way we error early if there are any severe errors.
    let mut cxs = vec![];
    let uses_default_packages = !workspace.workspace && workspace.package.is_empty();

    for package in packages {
        let _span = error_span!("", package = package.name.as_str()).entered();

        let manifest_path = ManifestPath::new(package.manifest_path.as_ref())?;
        let toml = manifest_path.get().read_to_string()?;

        let cfg_patch = config::read_package_config(&toml)?;

        let final_patch =
            workspace_package_config_patch.apply(&cfg_patch).apply(&cli.package_patch);

        if final_patch.bin.is_some() && final_patch.lib.is_some() {
            bail!("`lib` and `bin` are both set, you have to choose one or the other");
        }

        let cfg = final_patch.finish();

        let enabled_features =
            cfg.features.iter().filter(|&f| package.features.contains_key(f)).cloned().collect();

        let target = match &cfg.target_selection {
            Some(target_selection) => match target_selection {
                config::TargetSelection::Lib => {
                    package.targets.iter().find(|t| t.doc && is_lib_like(t))
                }
                config::TargetSelection::Bin(bin) => match bin {
                    Some(bin_name) => {
                        package.targets.iter().find(|t| t.doc && t.is_bin() && t.name == *bin_name)
                    }
                    None => package.targets.iter().find(|t| t.doc && t.is_bin()),
                },
            },
            None => {
                let lib = package.targets.iter().find(|t| t.doc && is_lib_like(t));
                let bin = || package.targets.iter().find(|t| t.doc && t.is_bin());
                lib.or_else(bin)
            }
        };

        let Some(target) = target else {
            continue;
        };

        let relative_readme_path = if let Some(path) = cfg.readme_path.as_deref() {
            path
        } else if let Some(path) = package.readme.as_deref() {
            path.as_std_path()
        } else {
            Path::new("README.md")
        };

        let readme_path = manifest_path.relative(relative_readme_path);

        let mut cmd = MetadataCommand::new();
        cmd.manifest_path(&package.manifest_path);

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

        cxs.push(PackageContext {
            cli,
            cfg,
            cfg_patch,
            package,
            target,
            enabled_features,
            manifest_path,
            readme_path,
            uses_default_packages,
            metadata,
            log: log.clone(),
        })
    }

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
                workspace: &cli.workspace_patch,
                package: &cli.package_patch,
            },
            workspace: WorkspaceAndPackageConfigPatch {
                workspace: &workspace_workspace_config_patch,
                package: &workspace_package_config_patch,
            },
        })
        .wrap_err("toml serialization failed")?;

        for cx in &cxs {
            let name = cx.package.name.as_str();

            out.push('\n');

            out.push_str(
                &toml::to_string(&PerPackage {
                    package: HashMap::from_iter([(name, &cx.cfg_patch)]),
                    resolved: HashMap::from_iter([(
                        name,
                        WorkspaceAndPackageConfig { workspace: &workspace, package: &cx.cfg },
                    )]),
                })
                .wrap_err("toml serialization failed")?,
            );
        }

        log.foreign_write_incoming();
        println!("{out}");
        return Ok(());
    }

    if cxs.is_empty() {
        let _span = workspace_package_config_patch
            .finish()
            .target_selection
            .map(|filter| error_span!("", %filter).entered());
        bail!("no target found to document");
    }

    check_version_control(&cxs)?;

    for cx in &cxs {
        run_package(cx);
    }

    Ok(())
}

// Modified from `fn check_version_control` in `rust-lang/cargo/src/cargo/ops/fix/mod.rs`.
fn check_version_control(cxs: &[PackageContext]) -> Result<()> {
    if cxs.is_empty() {
        return Ok(());
    }

    // bool: allow_staged
    let mut files: Vec<(&Path, bool)> = vec![];

    for cx in cxs {
        if cx.cfg.check || cx.cfg.allow_dirty {
            continue;
        }

        if cx.cfg.feature_into_crate {
            let path = cx.target.src_path.as_std_path();
            files.push((path, cx.cfg.allow_staged));
        }

        if cx.cfg.crate_into_readme {
            let path = cx.readme_path.full_path.as_path();
            files.push((path, cx.cfg.allow_staged));
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
        path.relative_to(cxs[0].metadata.workspace_root.as_std_path())
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

fn run_package(cx: &PackageContext) {
    let _span = (!cx.uses_default_packages || (*cx.metadata.workspace_default_members).len() > 1)
        .then(|| info_span!("", package = cx.package.name.as_str()).entered());

    if cx.cfg.feature_into_crate {
        task(cx, "feature documentation", "crate documentation", insert_features_into_docs);
    }

    if cx.cfg.crate_into_readme {
        task(cx, "crate documentation", "readme", insert_docs_into_readme);
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

struct PackageContext<'a> {
    cli: &'a Cli,
    cfg: PackageConfig,
    cfg_patch: PackageConfigPatch, // just for `--print-config`
    package: &'a Package,
    enabled_features: Vec<String>,
    manifest_path: ManifestPath,
    target: &'a Target,
    readme_path: RelativePath,
    uses_default_packages: bool,
    metadata: Metadata,
    log: PrettyLog,
}

struct ManifestPath(PathBuf);

impl ManifestPath {
    fn new(path: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        path.parent().ok_or_eyre("path has no parent")?;
        path.file_name().ok_or_eyre("path has no file name")?;
        Ok(Self(path))
    }

    fn get(&self) -> RelativePath {
        RelativePath {
            full_path: self.0.clone(),
            relative_to_manifest: self.0.file_name().unwrap().into(),
        }
    }

    fn relative(&self, relative: impl Into<PathBuf>) -> RelativePath {
        let relative_to_manifest = relative.into();

        RelativePath {
            full_path: self.0.parent().unwrap().join(&relative_to_manifest),
            relative_to_manifest,
        }
    }
}

// for better error messages when reading / writing files
struct RelativePath {
    full_path: PathBuf,
    relative_to_manifest: PathBuf,
}

impl RelativePath {
    fn read_to_string(&self) -> Result<String> {
        let _span = error_span!("", path = %self.full_path.display()).entered();
        let relative_path = self.relative_to_manifest.display();
        fs::read_to_string(&self.full_path)
            .with_context(|| format!("failed to read {relative_path}"))
    }

    fn write(&self, contents: &str) -> Result<()> {
        let _span = error_span!("", path = %self.full_path.display()).entered();
        let relative_path = self.relative_to_manifest.display();
        fs::write(&self.full_path, contents)
            .with_context(|| format!("failed to write {relative_path}"))
    }
}

fn task(cx: &PackageContext, from: &str, to: &str, f: fn(&PackageContext) -> Result<()>) {
    let task_name = if cx.cfg.check {
        format!("checking {from} in {to}")
    } else {
        format!("insert {from} into {to}")
    };

    let _span = info_span!("", task = task_name).entered();

    trace!("starting task");

    let start = Instant::now();

    if let Err(report) = f(cx) {
        let context = if cx.cfg.check {
            format!("checking {from} failed")
        } else {
            format!("could not {task_name}")
        };

        cx.log.print_report(&report.wrap_err(context));
    }

    trace!("finished in {:?}", start.elapsed());
}

fn insert_features_into_docs(cx: &PackageContext) -> Result<()> {
    let not_found_level = if cx.cfg.allow_missing_section { Level::WARN } else { Level::ERROR };

    let target_path = cx.target.src_path.as_std_path();
    let target_src = read_to_string(target_path)?;

    let Some(feature_docs_section) =
        edit_crate_docs::FeatureDocsSection::find(&target_src, &cx.cfg.feature_section_name)?
    else {
        let target_name = target_path
            .file_name()
            .map(|n| Path::new(n).display().to_string())
            .unwrap_or_else(|| "crate docs".into());

        let _span = info_span!("",
            path = %target_path.display(),
            section_name = cx.cfg.feature_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {target_name}")).with_severity(not_found_level);
    };

    let cargo_toml = cx.manifest_path.get().read_to_string()?;
    let hidden_features =
        cx.cfg.hidden_features.iter().map(|s| s.as_str()).collect::<HashSet<&str>>();

    let feature_docs =
        extract_feature_docs::extract(&cargo_toml, &cx.cfg.feature_label, &hidden_features)
            .wrap_err("failed to parse Cargo.toml")?;

    let new_target_src = feature_docs_section.replace(&feature_docs)?;

    if new_target_src != target_src {
        if cx.cfg.check {
            bail!("feature documentation is stale");
        }

        write(target_path, new_target_src.as_bytes())?;
    }

    Ok(())
}

fn insert_docs_into_readme(cx: &PackageContext) -> Result<()> {
    let not_found_level = if cx.cfg.allow_missing_section { Level::WARN } else { Level::ERROR };

    let readme_path = &cx.readme_path;
    let readme = readme_path.read_to_string().with_severity(not_found_level)?;

    let section_name = &cx.cfg.crate_section_name;
    let subsections = markdown::find_subsections(&readme, section_name)?;

    let new_readme = if !subsections.is_empty() {
        let crate_docs = extract_crate_docs::extract(cx)?;
        let [without_definitions, definitions] = markdown::extract_definitions(&crate_docs);

        let mut new_readme = StringReplacer::new(&readme);

        for (i, (section, name)) in subsections.into_iter().enumerate() {
            let replace_with_section = markdown::find_section(&without_definitions, &format!("{section_name} {name}")).ok_or_else(|| eyre!("\"{section_name}\" subsection \"{name}\" is contained in readme but missing from crate docs"))?;

            if i == 0 {
                let replace_with = &without_definitions[replace_with_section.content_span];
                new_readme.insert(section.span.end, format!("<!-- {section_name} {name} end -->"));
                new_readme.insert(section.span.end, &definitions);
                new_readme.insert(section.span.end, "\n");
                new_readme.replace(section.span.clone(), replace_with);
                new_readme
                    .insert(section.span.start, format!("<!-- {section_name} {name} start -->"));
            } else {
                let replace_with = &without_definitions[replace_with_section.span];
                new_readme.replace(section.span.clone(), replace_with);
            }
        }

        new_readme.finish()
    } else if let Some(section) = markdown::find_section(&readme, &cx.cfg.crate_section_name) {
        let crate_docs = extract_crate_docs::extract(cx)?;
        let mut new_readme = readme.clone();
        new_readme.replace_range(section.content_span, &format!("\n{crate_docs}\n"));
        new_readme
    } else {
        let relative_path = readme_path.relative_to_manifest.display();

        let _span = info_span!("",
            path = %readme_path.full_path.display(),
            section_name = cx.cfg.crate_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {relative_path}")).with_severity(not_found_level);
    };

    if readme != new_readme {
        if cx.cfg.check {
            bail!("crate documentation is stale");
        }

        readme_path.write(&new_readme)?;
    }

    Ok(())
}

fn read_to_string(path: &Path) -> Result<String> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    fs::read_to_string(path).with_context(|| format!("failed to read {file_name}"))
}

fn write(path: &Path, content: &[u8]) -> Result<()> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    fs::write(path, content).with_context(|| format!("failed to write to {file_name}"))
}
