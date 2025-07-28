#![doc = include_str!("../README.md")]

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
    collections::HashSet,
    ffi::{OsStr, OsString},
    fmt::{self, Write},
    fs, io,
    ops::Deref,
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
use tracing::{Level, error_span, info_span, trace};

use pretty_log::{PrettyLog, WithResultSeverity as _};

use crate::pretty_log::AnyWrite;

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

    /// Formatting of the feature label
    ///
    /// When inserting feature documentation into the crate documentation.
    #[arg(global = true, long, default_value = "**`{feature}`**")]
    feature_label: String,

    /// Feature documentation section name
    #[arg(global = true, long, value_name = "NAME", default_value = "feature documentation")]
    feature_section_name: String,

    /// Crate documentation section name
    #[arg(global = true, long, value_name = "NAME", default_value = "crate documentation")]
    crate_section_name: String,

    #[expect(rustdoc::bare_urls)]
    /// Link to the "latest" version on docs.rs
    ///
    /// For example https://docs.rs/my-crate/latest/my_crate/.
    /// This only affects workspace crates.
    #[arg(global = true, long, verbatim_doc_comment)]
    link_to_latest: bool,

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

    /// Coloring
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, long, value_name = "WHEN", value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

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
    #[arg(global = true, help_heading = heading::PACKAGE_SELECTION, long, value_name = "SPEC")]
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

    /// Which rustup toolchain to use when invoking rustdoc.
    ///
    /// Whenever you update your nightly toolchain this tool may also need to be
    /// updated to be compatible.
    ///
    /// With this argument you can choose a nightly version that is guaranteed to be compatible
    /// with the current version of this tool, like `nightly-2025-07-16`.
    #[arg(global = true, help_heading = heading::COMPILATION_OPTIONS, long, default_value = "nightly", verbatim_doc_comment)]
    toolchain: String,

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

impl Args {
    fn feature_section_enabled(&self) -> bool {
        self.command != Some(Command::CrateIntoReadme)
    }

    fn crate_section_enabled(&self) -> bool {
        self.command != Some(Command::FeatureIntoCrate)
    }
}

#[derive(clap::Subcommand, Clone, Copy, PartialEq, Eq)]
enum Command {
    // Only inserts feature documentation into crate documentation
    FeatureIntoCrate,
    // Only inserts crate documentation into the readme file
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

impl TargetSelection {
    fn select<'a>(&self, targets: &'a [Target]) -> Option<&'a Target> {
        if self.lib {
            targets.iter().find(|t| t.doc && t.is_lib())
        } else if let Some(bin) = self.bin.as_ref() {
            if let Some(bin) = bin.as_deref() {
                targets.iter().find(|t| t.doc && t.is_bin() && t.name == bin)
            } else {
                targets.iter().find(|t| t.doc && t.is_bin())
            }
        } else {
            let lib = targets.iter().find(|t| t.doc && t.is_lib());
            let bin = || targets.iter().find(|t| t.doc && t.is_bin());
            lib.or_else(bin)
        }
    }
}

impl fmt::Display for TargetSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.lib {
            f.write_str("--lib")
        } else if let Some(bin) = self.bin.as_ref() {
            f.write_str("--bin")?;

            if let Some(bin) = bin.as_deref() {
                f.write_char(' ')?;
                f.write_str(bin)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
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
    let mut args = parse_args();

    if args.quiet {
        args.quiet_cargo = true;
    }

    if args.allow_dirty {
        args.allow_staged = true;
    }

    // features are already comma separated, we still need to make them space separated
    args.features =
        args.features.iter().flat_map(|f| f.split(' ').map(|s| s.to_string())).collect();

    let stream: Box<dyn AnyWrite> = if args.quiet {
        Box::new(io::empty())
    } else {
        Box::new(anstream::AutoStream::new(
            std::io::stderr(),
            match args.color {
                ColorChoice::Auto => anstream::ColorChoice::Auto,
                ColorChoice::Always => anstream::ColorChoice::Always,
                ColorChoice::Never => anstream::ColorChoice::Never,
            },
        ))
    };

    let log = PrettyLog::new(stream);

    let log_level = if args.verbose { "trace" } else { "info" };
    log.install(&format!("cargo_insert_docs={log_level}"));

    if let Err(err) = try_main(&args, &log) {
        log.print_report(&err);
    }

    log.print_tally();

    if log.tally().errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn try_main(args: &Args, log: &PrettyLog) -> Result<()> {
    let mut cmd = MetadataCommand::new();

    if let Some(manifest_path) = args.manifest_path.as_deref() {
        cmd.manifest_path(manifest_path);
    }

    if args.no_default_features {
        cmd.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
    }

    if args.all_features {
        cmd.features(cargo_metadata::CargoOpt::AllFeatures);
    }

    if args.features.is_empty() {
        cmd.features(cargo_metadata::CargoOpt::SomeFeatures(args.features.clone()));
    }

    let metadata = cmd.exec()?;

    run(&BaseContext {
        args,
        metadata,
        log: log.clone(),
        uses_default_packages: !args.workspace && args.package.is_empty(),
    })
}

fn run(cx: &BaseContext) -> Result<()> {
    let mut packages: Vec<&Package> = if cx.args.workspace {
        cx.metadata.workspace_members.iter().map(|p| &cx.metadata[p]).collect()
    } else if cx.args.package.is_empty() {
        assert!(
            cx.metadata.workspace_default_members.is_available(),
            "to infer the current package, cargo of rust version 1.71 or higher is required"
        );

        if cx.metadata.workspace_default_members.is_available() {
            (*cx.metadata.workspace_default_members).iter().map(|p| &cx.metadata[p]).collect()
        } else {
            let cargo_toml = ManifestPath::new("Cargo.toml".as_ref())?.get().read_to_string()?;
            let package_name = manifest_package_name(&cargo_toml)
                .wrap_err("tried to read Cargo.toml to figure out package name")?;
            vec![find_package_by_name(cx, &package_name)?]
        }
    } else {
        find_packages_by_name(cx, &cx.args.package)?
    };

    let excluded_packages = cx
        .args
        .exclude
        .iter()
        .map(|name| find_package_by_name(cx, name))
        .collect::<Result<HashSet<_>, _>>()?;

    packages.retain(|id| !excluded_packages.contains(id));

    if packages.is_empty() {
        bail!("no packages selected");
    }

    // error if a feature is not available in any selected package
    {
        let all_available_features = packages
            .iter()
            .flat_map(|p| p.features.keys())
            .map(|s| s.as_str())
            .collect::<HashSet<&str>>();

        let unavailable_features = cx
            .args
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

    let mut cxs = vec![];

    for package in packages {
        let manifest_path = ManifestPath::new(package.manifest_path.as_ref())?;

        let enabled_features = cx
            .args
            .features
            .iter()
            .filter(|&f| package.features.contains_key(f))
            .cloned()
            .collect();

        let Some(target) = cx.args.target_selection.select(&package.targets) else {
            continue;
        };

        let relative_readme_path = if let Some(path) = cx.args.readme_path.as_deref() {
            path
        } else if let Some(path) = package.readme.as_deref() {
            path.as_std_path()
        } else {
            Path::new("README.md")
        };

        let readme_path = manifest_path.relative(relative_readme_path);

        cxs.push(Context {
            base: cx,
            package: PackageContext {
                package,
                target,
                enabled_features,
                manifest_path,
                readme_path,
            },
        })
    }

    if cxs.is_empty() {
        let filter = cx.args.target_selection.to_string();
        let _span = (!filter.is_empty()).then(|| error_span!("", filter).entered());
        bail!("no target found to document");
    }

    // Exit early if any affected file is dirty.
    check_version_control(cx, &cxs)?;

    for cx in &cxs {
        run_package(cx);
    }

    Ok(())
}

// Modified from `fn check_version_control` in `rust-lang/cargo/src/cargo/ops/fix/mod.rs`.
fn check_version_control(cx: &BaseContext, cxs: &[Context]) -> Result<()> {
    if cx.args.check || cx.args.allow_dirty {
        return Ok(());
    }

    let mut dirty_files = vec![];
    let mut staged_files = vec![];

    for cx in cxs {
        if cx.args.feature_section_enabled() {
            let lib_path = cx.package.target.src_path.as_std_path();

            let lib_path_display = lib_path
                .relative_to(cx.metadata.workspace_root.as_std_path())
                .map(|p| p.to_string())
                .unwrap_or_else(|_| lib_path.display().to_string());

            if let Some(status) = git::file_status(lib_path) {
                match status {
                    git::Status::Current => (),
                    git::Status::Staged => {
                        if !cx.args.allow_staged {
                            staged_files.push(lib_path_display);
                        }
                    }
                    git::Status::Dirty => {
                        if !cx.args.allow_dirty {
                            dirty_files.push(lib_path_display);
                        }
                    }
                }
            }
        }

        if cx.args.crate_section_enabled() {
            let readme_path = cx.package.readme_path.full_path.as_path();

            let readme_path_display = readme_path
                .relative_to(cx.metadata.workspace_root.as_std_path())
                .map(|p| p.to_string())
                .unwrap_or_else(|_| readme_path.display().to_string());

            if let Some(status) = git::file_status(readme_path) {
                match status {
                    git::Status::Current => (),
                    git::Status::Staged => {
                        if !cx.args.allow_staged {
                            staged_files.push(readme_path_display);
                        }
                    }
                    git::Status::Dirty => {
                        if !cx.args.allow_dirty {
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

    if cx.args.feature_section_enabled() {
        task(cx, "feature documentation", "crate documentation", insert_features_into_docs);
    }

    if cx.args.crate_section_enabled() {
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

fn find_packages_by_name<'a>(
    cx: &'a BaseContext,
    package_names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Vec<&'a Package>> {
    package_names.into_iter().map(|name| find_package_by_name(cx, name.as_ref())).collect()
}

// support package SPECs
fn find_package_by_name<'a>(cx: &'a BaseContext, package_name: &str) -> Result<&'a Package> {
    for workspace_member in &cx.metadata.workspace_members {
        let package = &cx.metadata[workspace_member];

        if package.name.as_str() == package_name {
            return Ok(package);
        }
    }

    bail!("no package named \"{package_name}\" found")
}

struct BaseContext<'a> {
    args: &'a Args,
    log: PrettyLog,
    metadata: Metadata,
    uses_default_packages: bool,
}

struct Context<'a> {
    base: &'a BaseContext<'a>,
    package: PackageContext<'a>,
}

impl<'a> Deref for Context<'a> {
    type Target = BaseContext<'a>;

    fn deref(&self) -> &Self::Target {
        self.base
    }
}

struct PackageContext<'a> {
    package: &'a Package,
    enabled_features: Vec<String>,
    manifest_path: ManifestPath,
    target: &'a Target,
    readme_path: RelativePath,
}

impl Deref for PackageContext<'_> {
    type Target = Package;

    fn deref(&self) -> &Self::Target {
        self.package
    }
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
    let task_name = if cx.args.check {
        format!("checking {from} in {to}")
    } else {
        format!("insert {from} into {to}")
    };

    let _span = info_span!("", task = task_name).entered();

    trace!("starting task");

    let start = Instant::now();

    if let Err(report) = f(cx) {
        let context = if cx.args.check {
            format!("checking {from} failed")
        } else {
            format!("could not {task_name}")
        };

        cx.log.print_report(&report.wrap_err(context));
    }

    trace!("finished in {:?}", start.elapsed());
}

fn insert_features_into_docs(cx: &Context) -> Result<()> {
    let not_found_level = if cx.args.allow_missing_section { Level::WARN } else { Level::ERROR };

    let target_path = cx.package.target.src_path.as_std_path();
    let target_src = read_to_string(target_path)?;

    let Some(feature_docs_section) =
        edit_crate_docs::FeatureDocsSection::find(&target_src, &cx.args.feature_section_name)?
    else {
        let target_name = target_path
            .file_name()
            .map(|n| Path::new(n).display().to_string())
            .unwrap_or_else(|| "crate docs".into());

        let _span = info_span!("",
            path = %target_path.display(),
            section_name = cx.args.feature_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {target_name}")).with_severity(not_found_level);
    };

    let cargo_toml = cx.package.manifest_path.get().read_to_string()?;

    let feature_docs = extract_feature_docs::extract(&cargo_toml, &cx.args.feature_label)
        .wrap_err("failed to parse Cargo.toml")?;

    let new_target_src = feature_docs_section.replace(&feature_docs)?;

    if new_target_src != target_src {
        if cx.args.check {
            bail!("feature documentation is stale");
        }

        write(target_path, new_target_src.as_bytes())?;
    }

    Ok(())
}

fn insert_docs_into_readme(cx: &Context) -> Result<()> {
    let not_found_level = if cx.args.allow_missing_section { Level::WARN } else { Level::ERROR };

    let readme_path = &cx.package.readme_path;
    let readme = readme_path.read_to_string().with_severity(not_found_level)?;

    let Some(section) = markdown::find_section(&readme, &cx.args.crate_section_name) else {
        let relative_path = readme_path.relative_to_manifest.display();

        let _span = info_span!("",
            path = %readme_path.full_path.display(),
            section_name = cx.args.crate_section_name,
        )
        .entered();

        return Err(eyre!("section not found in {relative_path}")).with_severity(not_found_level);
    };

    let crate_docs = extract_crate_docs::extract(cx)?;

    let mut new_readme = readme.clone();
    new_readme.replace_range(section, &format!("\n{crate_docs}\n"));

    if readme != new_readme {
        if cx.args.check {
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
