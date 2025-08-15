#![doc = include_str!("../README.md")]
#![allow(
    // ifs are intentionally uncollapsed to make the logic clearer
    clippy::collapsible_else_if,
)]

mod config;
mod edit_crate_docs;
mod extract_crate_docs;
mod extract_feature_docs;
mod git;
mod markdown;
mod pretty_log;
mod rustdoc_json;
#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use cargo_metadata::{Metadata, MetadataCommand, Package, Target};
use clap::{Parser, ValueEnum};
use clap_cargo::style::CLAP_STYLING;
use color_eyre::eyre::{OptionExt, Result, WrapErr as _, bail, eyre};
use mimalloc::MiMalloc;
use relative_path::PathExt;
use serde::Serialize;
use tracing::{Level, error_span, info_span, trace};

use pretty_log::{PrettyLog, WithResultSeverity as _};

use crate::{
    config::{
        ArgsConfig, PackageConfig, PackageConfigPatch, WorkspaceConfig, WorkspaceConfigPatch,
    },
    pretty_log::AnyWrite,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod heading {
    pub const PACKAGE_SELECTION: &str = "Package Selection";
    pub const TARGET_SELECTION: &str = "Target Selection";
    pub const FEATURE_SELECTION: &str = "Feature Selection";
    pub const COMPILATION_OPTIONS: &str = "Compilation Options";
    pub const MANIFEST_OPTIONS: &str = "Manifest Options";
    pub const ERROR_BEHAVIOR: &str = "Error Behavior";
    pub const MESSAGE_OPTIONS: &str = "Message Options";
    pub const MODE_SELECTION: &str = "Mode Selection";
    pub const CARGO_DOC_OPTIONS: &str = "Cargo Doc Options";
}

#[derive(Parser)]
#[command(
    version,
    about = "Inserts crate docs into a readme file and feature docs into the crate docs.",
    long_about = "\
        Inserts feature documentation into the crate documentation and the crate documentation into the readme.\n\n\
        Website: https://github.com/bluurryy/cargo-insert-docs",
    bin_name = "cargo insert-docs",
    styles = CLAP_STYLING
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Formatting of the feature label [default: "**`{feature}`**"]
    ///
    /// When inserting feature documentation into the crate documentation.
    #[arg(global = true, long)]
    feature_label: Option<String>,

    /// Feature documentation section name [default: "feature documentation"]
    #[arg(global = true, long, value_name = "NAME")]
    feature_section_name: Option<String>,

    /// Crate documentation section name [default: "crate documentation"]
    #[arg(global = true, long, value_name = "NAME")]
    crate_section_name: Option<String>,

    #[expect(rustdoc::bare_urls)]
    /// Link to the "latest" version on docs.rs
    ///
    /// For example https://docs.rs/my-crate/latest/my_crate/.
    /// This only affects workspace crates.
    #[arg(global = true, long, verbatim_doc_comment)]
    link_to_latest: bool,

    /// Prints a supported nightly toolchain
    #[arg(global = true, long)]
    print_supported_toolchain: bool,

    /// Prints configuration values and their sources for debugging
    #[arg(global = true, long)]
    print_config: bool,

    /// Document private items
    #[arg(global = true, help_heading = heading::CARGO_DOC_OPTIONS, long)]
    document_private_items: bool,

    /// Don't build documentation for dependencies
    #[arg(global = true, help_heading = heading::CARGO_DOC_OPTIONS, long)]
    no_deps: bool,

    /// Runs in 'check' mode, not writing to files but erroring if something is out of date
    ///
    /// Exits with 0 if the documentation is up to date.
    /// Exits with 1 if the documentation is stale or if any errors occured.
    #[arg(global = true, help_heading = heading::MODE_SELECTION, long, verbatim_doc_comment)]
    check: bool,

    /// Don't error when a section is missing
    #[arg(global = true, help_heading = heading::ERROR_BEHAVIOR, long)]
    allow_missing_section: bool,

    /// Insert documentation even if the affected file is dirty or has staged changes
    #[arg(global = true, help_heading = heading::ERROR_BEHAVIOR, long)]
    allow_dirty: bool,

    /// Insert documentation even if the affected file has staged changes
    #[arg(global = true, help_heading = heading::ERROR_BEHAVIOR, long)]
    allow_staged: bool,

    /// Coloring [default: "auto"]
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, long, value_name = "WHEN", value_enum)]
    color: Option<ColorChoice>,

    /// Print more verbose messages
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, long, short = 'v')]
    verbose: bool,

    /// Do not print anything
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, long, short = 'q')]
    quiet: bool,

    /// Do not print cargo log messages
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, long)]
    quiet_cargo: bool,

    /// Package(s) to document
    #[arg(global = true, help_heading = heading::PACKAGE_SELECTION, long, short = 'p', value_name = "SPEC")]
    package: Vec<String>,

    /// Document all packages in the workspace
    #[arg(global = true, help_heading = heading::PACKAGE_SELECTION, long)]
    workspace: bool,

    /// Exclude package(s) from documenting
    #[arg(global = true, help_heading = heading::PACKAGE_SELECTION, long, value_name = "SPEC", requires = "workspace")]
    exclude: Vec<String>,

    /// Space or comma separated list of features to activate
    #[arg(global = true, help_heading = heading::FEATURE_SELECTION, long, short = 'F', value_delimiter = ',')]
    features: Vec<String>,

    /// Activate all available features
    #[arg(global = true, help_heading = heading::FEATURE_SELECTION, long)]
    all_features: bool,

    /// Do not activate the `default` feature
    #[arg(global = true, help_heading = heading::FEATURE_SELECTION, long)]
    no_default_features: bool,

    #[command(flatten)]
    target_selection: TargetSelection,

    /// Which rustup toolchain to use when invoking rustdoc [default: "nightly-2025-08-02"]
    ///
    /// The default value is a toolchain that is known to be compatible with
    /// this version of `cargo-insert-docs`.
    #[arg(global = true, help_heading = heading::COMPILATION_OPTIONS, long, verbatim_doc_comment)]
    toolchain: Option<String>,

    /// Target triple to document
    #[arg(global = true, help_heading = heading::COMPILATION_OPTIONS, long, value_name = "TRIPLE")]
    target: Option<String>,

    /// Directory for all generated artifacts
    #[arg(global = true, help_heading = heading::COMPILATION_OPTIONS, long, value_name = "DIRECTORY")]
    target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[arg(global = true, help_heading = heading::MANIFEST_OPTIONS, long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Readme path relative to the package manifest
    ///
    /// This defaults to the `readme` field as specified in the `Cargo.toml`.
    #[arg(global = true, help_heading = heading::MANIFEST_OPTIONS, long, value_name = "PATH")]
    readme_path: Option<PathBuf>,
}

#[derive(clap::Subcommand, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Only inserts feature documentation into crate documentation
    FeatureIntoCrate,
    /// Only inserts crate documentation into the readme file
    CrateIntoReadme,
}

#[derive(clap::Args)]
#[group(multiple = false)]
struct TargetSelection {
    /// Document only library targets
    #[arg(help_heading = heading::TARGET_SELECTION, long)]
    lib: bool,

    /// Document only the specified binary
    #[arg(help_heading = heading::TARGET_SELECTION, long, value_name = "NAME")]
    bin: Option<Option<String>>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// <https://doc.rust-lang.org/cargo/reference/external-tools.html#custom-subcommands>
///
/// When executing `cargo-insert-docs` as a cargo subcommand
/// the first argument will be filename as usual.
///
/// The second argument will be `insert-docs`.
///
/// To be able to run `cargo-insert-docs` directly and as subcommand
/// we need to filter out that extra `insert-docs` argument.
///
/// To support any executable name and not just the hardcoded "insert-docs"
/// we parse the filename, remove the "cargo-" prefix and the ".exe" suffix
/// to get the name of the second argument.
fn parse_args() -> Args {
    let command = std::env::args_os().next().expect("first argument is missing");
    let command = subcommand_name(command.as_os_str());
    let command = command.as_ref();

    let args_os = std::env::args_os()
        .enumerate()
        .filter(|(index, arg)| *index != 1 || Some(arg) != command)
        .map(|(_, arg)| arg);

    Args::parse_from(args_os)
}

fn subcommand_name(bin: &OsStr) -> Option<OsString> {
    Some(
        Path::new(bin)
            .file_name()?
            .to_string_lossy()
            .strip_prefix("cargo-")?
            .strip_suffix(std::env::consts::EXE_SUFFIX)?
            .to_string()
            .into(),
    )
}

fn main() -> ExitCode {
    let args = parse_args();
    let args = ArgsConfig::from_args(&args);

    if args.cli.print_supported_toolchain {
        println!("{}", config::DEFAULT_TOOLCHAIN);
        return ExitCode::SUCCESS;
    }

    let stream: Box<dyn AnyWrite> = if args.cli.quiet {
        Box::new(io::empty())
    } else {
        Box::new(anstream::AutoStream::new(
            std::io::stderr(),
            match args.cli.color {
                ColorChoice::Auto => anstream::ColorChoice::Auto,
                ColorChoice::Always => anstream::ColorChoice::Always,
                ColorChoice::Never => anstream::ColorChoice::Never,
            },
        ))
    };

    let log = PrettyLog::new(stream);
    log.print_source_info(args.cli.verbose);

    let log_level = if args.cli.verbose { "trace" } else { "info" };
    log.install(&format!("cargo_insert_docs={log_level}"));

    if let Err(err) = try_main(&args, &log) {
        log.print_report(&err);
    }

    log.print_tally();

    if log.tally().errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn try_main(args: &ArgsConfig, log: &PrettyLog) -> Result<()> {
    let mut cmd = MetadataCommand::new();

    if let Some(manifest_path) = args.cli.manifest_path.as_deref() {
        cmd.manifest_path(manifest_path);
    }

    let metadata = cmd.exec()?;
    let (workspace_workspace_config_patch, workspace_package_config_patch) =
        config::read_workspace_config(&metadata.workspace_metadata)?;

    let workspace = workspace_workspace_config_patch.apply(&args.workspace_patch).finish();

    let mut packages: Vec<&Package> = if workspace.workspace {
        metadata.workspace_members.iter().map(|p| &metadata[p]).collect()
    } else if workspace.package.is_empty() {
        assert!(
            metadata.workspace_default_members.is_available(),
            "to infer the current package, cargo of rust version 1.71 or higher is required"
        );

        // FIXME: just refuse to run if the workspace default members are not available
        // it has been available since 1.71
        if metadata.workspace_default_members.is_available() {
            (*metadata.workspace_default_members).iter().map(|p| &metadata[p]).collect()
        } else {
            let cargo_toml = ManifestPath::new("Cargo.toml".as_ref())?.get().read_to_string()?;
            let package_name = manifest_package_name(&cargo_toml)
                .wrap_err("tried to read Cargo.toml to figure out package name")?;
            vec![find_package_by_name(&metadata, &package_name)?]
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
    if !args.cli.print_config {
        let pkg = workspace_package_config_patch.clone().apply(&args.package_patch).finish();

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
        let manifest_path = ManifestPath::new(package.manifest_path.as_ref())?;
        let toml = manifest_path.get().read_to_string()?;

        let cfg_patch = config::read_package_config(&toml)?;

        let final_patch =
            workspace_package_config_patch.apply(&cfg_patch).apply(&args.package_patch);

        if final_patch.bin.is_some() && final_patch.lib.is_some() {
            bail!("`lib` and `bin` are both set, you have to choose one or the other");
        }

        let cfg = final_patch.finish();

        let enabled_features =
            cfg.features.iter().filter(|&f| package.features.contains_key(f)).cloned().collect();

        let target = match &cfg.target_selection {
            Some(target_selection) => match target_selection {
                config::TargetSelection::Lib => {
                    package.targets.iter().find(|t| t.doc && t.is_lib())
                }
                config::TargetSelection::Bin(bin) => match bin {
                    Some(bin_name) => {
                        package.targets.iter().find(|t| t.doc && t.is_bin() && t.name == *bin_name)
                    }
                    None => package.targets.iter().find(|t| t.doc && t.is_bin()),
                },
            },
            None => {
                let lib = package.targets.iter().find(|t| t.doc && t.is_lib());
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

        cxs.push(Context {
            args,
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

    if args.cli.print_config {
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
                workspace: &args.workspace_patch,
                package: &args.package_patch,
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
fn check_version_control(cxs: &[Context]) -> Result<()> {
    let mut dirty_files = vec![];
    let mut staged_files = vec![];

    for cx in cxs {
        if cx.cfg.check || cx.cfg.allow_dirty {
            continue;
        }

        if cx.cfg.feature_into_crate {
            let lib_path = cx.target.src_path.as_std_path();

            let lib_path_display = lib_path
                .relative_to(cx.metadata.workspace_root.as_std_path())
                .map(|p| p.to_string())
                .unwrap_or_else(|_| lib_path.display().to_string());

            if let Some(status) = git::file_status(lib_path) {
                match status {
                    git::Status::Current => (),
                    git::Status::Staged => {
                        if !cx.cfg.allow_staged {
                            staged_files.push(lib_path_display);
                        }
                    }
                    git::Status::Dirty => {
                        if !cx.cfg.allow_dirty {
                            dirty_files.push(lib_path_display);
                        }
                    }
                }
            }
        }

        if cx.cfg.crate_into_readme {
            let readme_path = cx.readme_path.full_path.as_path();

            let readme_path_display = readme_path
                .relative_to(cx.metadata.workspace_root.as_std_path())
                .map(|p| p.to_string())
                .unwrap_or_else(|_| readme_path.display().to_string());

            if let Some(status) = git::file_status(readme_path) {
                match status {
                    git::Status::Current => (),
                    git::Status::Staged => {
                        if !cx.cfg.allow_staged {
                            staged_files.push(readme_path_display);
                        }
                    }
                    git::Status::Dirty => {
                        if !cx.cfg.allow_dirty {
                            dirty_files.push(readme_path_display);
                        }
                    }
                }
            }
        }
    }

    if dirty_files.is_empty() && staged_files.is_empty() {
        return Ok(());
    }

    let mut files_list = String::new();

    for file in dirty_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (dirty)\n");
    }
    for file in staged_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (staged)\n");
    }

    bail!(
        "the working directory of this package has uncommitted changes, and \n\
            `cargo fix` can potentially perform destructive changes; if you'd \n\
            like to suppress this error pass `--allow-dirty`, \n\
            or commit the changes to these files:\n\
            \n\
            {files_list}\n\
         "
    );
}

fn run_package(cx: &Context) {
    let _span = (!cx.uses_default_packages || (*cx.metadata.workspace_default_members).len() > 1)
        .then(|| info_span!("", package = cx.package.name.as_str()).entered());

    if cx.cfg.feature_into_crate {
        task(cx, "feature documentation", "crate documentation", insert_features_into_docs);
    }

    if cx.cfg.crate_into_readme {
        task(cx, "crate documentation", "readme", insert_docs_into_readme);
    }
}

fn manifest_package_name(cargo_toml: &str) -> Result<String> {
    let doc = toml_edit::Document::parse(cargo_toml)?;

    fn inner<'a>(doc: &'a toml_edit::Document<&'a str>) -> Option<&'a str> {
        doc.get("package")?.as_table_like()?.get("name")?.as_str()
    }

    inner(&doc).map(|s| s.to_string()).ok_or_eyre("Cargo.toml has no `package.name` field")
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

struct Context<'a> {
    args: &'a ArgsConfig,
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

fn task(cx: &Context, from: &str, to: &str, f: fn(&Context) -> Result<()>) {
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

fn insert_features_into_docs(cx: &Context) -> Result<()> {
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

    let feature_docs = extract_feature_docs::extract(&cargo_toml, &cx.cfg.feature_label)
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

fn insert_docs_into_readme(cx: &Context) -> Result<()> {
    let not_found_level = if cx.cfg.allow_missing_section { Level::WARN } else { Level::ERROR };

    let readme_path = &cx.readme_path;
    let readme = readme_path.read_to_string().with_severity(not_found_level)?;

    let section_name = &cx.cfg.crate_section_name;
    let subsections = markdown::find_subsections(&readme, section_name)?;

    let new_readme = if !subsections.is_empty() {
        let crate_docs = extract_crate_docs::extract(cx)?;
        let mut new_readme = readme.clone();

        for (section, name) in subsections.into_iter().rev() {
            let replace_with_section = markdown::find_section(&crate_docs, &format!("{section_name} {name}")).ok_or_else(|| eyre!("\"{section_name}\" subsection \"{name}\" is contained in readme but missing from crate docs"))?;
            let replace_with = &crate_docs[replace_with_section.span];
            new_readme.replace_range(section.span, replace_with);
        }

        new_readme
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
