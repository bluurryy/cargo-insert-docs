#![doc = include_str!("../README.md")]

mod edit_crate_docs;
mod extract_crate_docs;
mod extract_feature_docs;
mod git;
mod markdown;
mod pretty_log;
#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use cargo_metadata::{Metadata, MetadataCommand, PackageId};
use clap::Parser;
use clap_cargo::style::CLAP_STYLING;
use color_eyre::eyre::{OptionExt, Result, WrapErr as _, bail, eyre};
use mimalloc::MiMalloc;
use relative_path::PathExt;
use tracing::{Level, error_span, info_span, trace};

use pretty_log::{PrettyLog, WithResultSeverity as _};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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
    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH", default_value = "Cargo.toml")]
    manifest_path: PathBuf,

    /// Readme path relative to the package manifest
    #[arg(long, value_name = "PATH", default_value = "README.md")]
    readme_path: PathBuf,

    /// Activate all available features
    #[arg(long)]
    all_features: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,

    /// Space or comma separated list of features to activate
    #[arg(long, short = 'F', value_delimiter = ',')]
    features: Vec<String>,

    /// Formatting of the feature label
    ///
    /// When inserting feature documentation into the crate documentation.
    #[arg(long, default_value = "**`{feature}`**")]
    feature_label: String,

    /// Name of the feature documentation section
    #[arg(long, value_name = "SECTION_NAME", default_value = "feature documentation")]
    feature_docs_section: String,

    /// Name of the crate documentation section
    #[arg(long, value_name = "SECTION_NAME", default_value = "crate documentation")]
    crate_docs_section: String,

    /// Disables inserting the feature documentation into the crate documentation
    #[arg(long)]
    no_feature_docs: bool,

    /// Disables inserting the crate documentation into the readme
    #[arg(long)]
    no_crate_docs: bool,

    /// Errors instead of printing a warning when a documentation section was
    /// not found.
    ///
    /// Implies `--strict-feature-docs` and `--strict-crate-docs`.
    #[arg(long)]
    strict: bool,

    /// Errors instead of printing a warning when a feature documentation section
    /// was not found in the crate documentation.
    #[arg(long)]
    strict_feature_docs: bool,

    /// Errors instead of printing a warning when a crate documentation section
    /// was not found in the readme.
    #[arg(long)]
    strict_crate_docs: bool,

    /// Package(s) to document
    #[arg(long, short = 'p', value_name = "PACKAGE")]
    package: Vec<String>,

    /// Document all packages in the workspace
    #[arg(long)]
    workspace: bool,

    /// Exclude package(s) from documenting
    #[arg(long, value_name = "PACKAGE")]
    exclude: Vec<String>,

    /// Which rustup toolchain to use when invoking rustdoc.
    ///
    /// Whenever you update your nightly toolchain this tool may also need to be
    /// updated to be compatible.
    ///
    /// With this argument you can choose a nightly version that is guaranteed to be compatible
    /// with the current version of this tool, like `nightly-2025-07-16`.
    #[arg(long, default_value = "nightly", verbatim_doc_comment)]
    toolchain: String,

    /// Target triple to document
    #[arg(long, value_name = "TRIPLE")]
    target: Option<String>,

    /// Document private items
    #[arg(long)]
    document_private_items: bool,

    #[expect(rustdoc::bare_urls)]
    /// Link to the "latest" version on docs.rs
    ///
    /// For example https://docs.rs/my-crate/latest/my_crate/.
    /// This only affects workspace crates.
    #[arg(long, verbatim_doc_comment)]
    link_to_latest: bool,

    /// Print more verbose messages
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Do not print log messages
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Do not print cargo log messages
    #[arg(long)]
    quiet_cargo: bool,

    /// Insert documentation even if an affected file is uncommitted
    #[arg(long, short = 'f')]
    force: bool,

    /// Runs in 'check' mode
    ///
    /// Exits with 0 if the documentation is up to date.
    /// Exits with 1 if the documentation is stale or if any errors occured.
    #[arg(long, verbatim_doc_comment)]
    check: bool,
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

    if args.strict {
        args.strict_feature_docs = true;
        args.strict_crate_docs = true;
    }

    // features are already comma separated, we still need to make them space separated
    args.features =
        args.features.iter().flat_map(|f| f.split(' ').map(|s| s.to_string())).collect();

    let log = PrettyLog::new(if args.quiet {
        Box::new(io::empty())
    } else {
        Box::new(anstream::stderr())
    });

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

    cmd.manifest_path(&args.manifest_path);

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

    run(&BaseContext { args, metadata, log: log.clone() })
}

fn run(cx: &BaseContext) -> Result<()> {
    let is_explicit_package;

    let mut package_names: Vec<String> = if cx.args.workspace {
        is_explicit_package = true;
        cx.metadata
            .workspace_members
            .iter()
            .map(|id| &cx.metadata[id])
            .map(|p| p.name.to_string())
            .collect()
    } else if cx.args.package.is_empty() {
        is_explicit_package = false;
        let cargo_toml = ManifestPath::new(&cx.args.manifest_path)?.get().read_to_string()?;
        let package = manifest_package_name(&cargo_toml)
            .wrap_err("tried to read Cargo.toml to figure out package name")?;
        vec![package]
    } else {
        is_explicit_package = true;
        cx.args.package.clone()
    };

    let excluded_package_names = cx.args.exclude.iter().collect::<HashSet<_>>();
    package_names.retain(|name| !excluded_package_names.contains(name));

    let mut packages = vec![];

    // resolve package ids
    for package_name in package_names {
        let id = find_package_by_name(cx, &package_name)?;
        packages.push(id);
    }

    // error if a feature is not available in any selected package
    {
        let all_available_features = packages
            .iter()
            .flat_map(|id| cx.metadata[id].features.keys())
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

    // error if no package has a lib target
    if !packages.iter().any(|id| cx.metadata[id].targets.iter().any(|t| t.is_lib())) {
        bail!("no selected package contains a lib target");
    }

    let mut contexts = vec![];

    for id in packages {
        let package = &cx.metadata[&id];
        let manifest_path = ManifestPath::new(package.manifest_path.as_ref())?;
        let enabled_features = cx
            .args
            .features
            .iter()
            .filter(|&f| package.features.contains_key(f))
            .cloned()
            .collect();

        if !package.targets.iter().any(|t| t.is_lib()) {
            // we can only work with lib targets
            continue;
        }

        contexts.push(Context {
            base: cx,
            package: PackageContext {
                id,
                enabled_features,
                manifest_path,
                is_explicit: is_explicit_package,
            },
        })
    }

    // Exit early if any affected file is dirty.
    if !cx.args.check && !cx.args.force {
        let mut dirty = vec![];

        for cx in &contexts {
            dirty.extend(dirty_files(cx)?);
        }

        if !dirty.is_empty() {
            let _span = error_span!(
                "",
                info = "this is to prevent overwriting changes you may have made to a section",
                help = "use the `--force` argument to insert docs anyway",
            )
            .entered();
            bail!("uncommitted changes detected in affected files:\n{}", dirty.join("\n"))
        }
    }

    for cx in &contexts {
        run_package(cx);
    }

    Ok(())
}

fn dirty_files(cx: &Context) -> Result<Vec<String>> {
    let mut dirty = vec![];

    if !cx.args.no_feature_docs {
        let lib_path = cx.lib_path()?;

        if git::is_file_dirty(lib_path).unwrap_or(false) {
            dirty.push(
                lib_path
                    .relative_to(cx.metadata.workspace_root.as_std_path())
                    .map(|p| p.to_string())
                    .unwrap_or_else(|_| lib_path.display().to_string()),
            );
        }
    }

    if !cx.args.no_crate_docs {
        let readme_path = cx.readme_path().full_path;

        if git::is_file_dirty(&readme_path).unwrap_or(false) {
            dirty.push(
                readme_path
                    .relative_to(cx.metadata.workspace_root.as_std_path())
                    .map(|p| p.to_string())
                    .unwrap_or_else(|_| readme_path.display().to_string()),
            );
        }
    }

    Ok(dirty)
}

fn run_package(cx: &Context) {
    let _span = cx
        .package
        .is_explicit
        .then(|| info_span!("", package = cx.metadata[&cx.package.id].name.as_str()).entered());

    if !cx.args.no_feature_docs {
        task(cx, "feature documentation", "crate documentation", insert_features_into_docs);
    }

    if !cx.args.no_crate_docs {
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

fn find_package_by_name(cx: &BaseContext, package_name: &str) -> Result<PackageId> {
    for workspace_member in &cx.metadata.workspace_members {
        if cx.metadata[workspace_member].name.as_str() == package_name {
            return Ok(workspace_member.clone());
        }
    }

    bail!("no package named \"{package_name}\" found")
}

struct BaseContext<'a> {
    args: &'a Args,
    metadata: Metadata,
    log: PrettyLog,
}

struct Context<'a> {
    base: &'a BaseContext<'a>,
    package: PackageContext,
}

impl Context<'_> {
    fn lib_path(&self) -> Result<&Path> {
        Ok(self.metadata[&self.package.id]
            .targets
            .iter()
            .find(|target| target.is_lib())
            .ok_or_eyre("the selected package contains no lib target")?
            .src_path
            .as_ref())
    }

    fn readme_path(&self) -> RelativePath {
        self.package.manifest_path.relative(&self.args.readme_path)
    }
}

impl<'a> Deref for Context<'a> {
    type Target = BaseContext<'a>;

    fn deref(&self) -> &Self::Target {
        self.base
    }
}

struct PackageContext {
    id: PackageId,
    enabled_features: Vec<String>,
    manifest_path: ManifestPath,
    is_explicit: bool,
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
    let not_found_level = if cx.args.strict_feature_docs { Level::ERROR } else { Level::WARN };

    let lib_path = cx.lib_path()?;
    let lib = read_to_string(lib_path)?;

    let Some(feature_docs_section) =
        edit_crate_docs::FeatureDocsSection::find(&lib, &cx.args.feature_docs_section)?
    else {
        let lib_name = lib_path
            .file_name()
            .map(|n| Path::new(n).display().to_string())
            .unwrap_or_else(|| "crate docs".into());

        let _span = info_span!("",
            path = %lib_path.display(),
            section_name = cx.args.feature_docs_section,
        )
        .entered();

        return Err(eyre!("section not found in {lib_name}")).with_severity(not_found_level);
    };

    let cargo_toml = cx.package.manifest_path.get().read_to_string()?;

    let feature_docs = extract_feature_docs::extract(&cargo_toml, &cx.args.feature_label)
        .wrap_err("failed to parse Cargo.toml")?;

    let new_lib = feature_docs_section.replace(&feature_docs)?;

    if new_lib != lib {
        if cx.args.check {
            bail!("feature documentation is stale");
        }

        write(lib_path, new_lib.as_bytes())?;
    }

    Ok(())
}

fn insert_docs_into_readme(cx: &Context) -> Result<()> {
    let not_found_level = if cx.args.strict_crate_docs { Level::ERROR } else { Level::WARN };

    let readme_path = cx.package.manifest_path.relative(&cx.args.readme_path);
    let readme = readme_path.read_to_string().with_severity(not_found_level)?;

    let Some(section) = markdown::find_section(&readme, &cx.args.crate_docs_section) else {
        let relative_path = readme_path.relative_to_manifest.display();

        let _span = info_span!("",
            path = %readme_path.full_path.display(),
            section_name = cx.args.crate_docs_section,
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
