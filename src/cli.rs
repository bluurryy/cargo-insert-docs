use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use clap_cargo::style::CLAP_STYLING;

use crate::config::{BoolOrString, CliConfig, PackageConfigPatch, WorkspaceConfigPatch};

pub struct Cli {
    pub cfg: CliConfig,
    pub workspace_patch: WorkspaceConfigPatch,
    pub package_patch: PackageConfigPatch,
}

impl Cli {
    pub fn parse() -> Self {
        Self::from_args(&parse_args())
    }

    fn from_args(args: &Args) -> Self {
        let Args {
            // cli
            print_supported_toolchain,
            color,
            verbose,
            quiet,
            quiet_cargo,
            ref manifest_path,
            print_config,
            // workspace
            ref package,
            workspace,
            ref exclude,
            // package
            command,
            ref feature_label,
            ref feature_section_name,
            ref crate_section_name,
            shrink_headings,
            link_to_latest,
            document_private_items,
            no_deps,
            check,
            allow_missing_section,
            allow_dirty,
            allow_staged,
            ref features,
            all_features,
            no_default_features,
            ref hidden_features,
            ref target_selection,
            ref toolchain,
            ref target,
            ref target_dir,
            ref readme_path,
            ..
        } = *args;

        Self {
            cfg: CliConfig {
                print_supported_toolchain,
                print_config,
                color: match color.unwrap_or(ColorChoice::Auto) {
                    ColorChoice::Auto => anstream::ColorChoice::Auto,
                    ColorChoice::Always => anstream::ColorChoice::Always,
                    ColorChoice::Never => anstream::ColorChoice::Never,
                },
                verbose,
                quiet,
                quiet_cargo: quiet || quiet_cargo,
                manifest_path: manifest_path.clone(),
            },
            workspace_patch: WorkspaceConfigPatch {
                package: (!package.is_empty()).then(|| package.clone()),
                workspace: workspace.then_some(true),
                exclude: (!exclude.is_empty()).then(|| exclude.clone()),
            },
            package_patch: PackageConfigPatch {
                feature_into_crate: command.map(|c| c == Command::FeatureIntoCrate),
                crate_into_readme: command.map(|c| c == Command::CrateIntoReadme),
                feature_label: feature_label.clone(),
                feature_section_name: feature_section_name.clone(),
                crate_section_name: crate_section_name.clone(),
                shrink_headings,
                link_to_latest: link_to_latest.then_some(true),
                document_private_items: document_private_items.then_some(true),
                no_deps: no_deps.then_some(true),
                check: check.then_some(true),
                allow_missing_section: allow_missing_section.then_some(true),
                allow_dirty: allow_dirty.then_some(true),
                allow_staged: allow_staged.then_some(true),
                features: (!features.is_empty()).then(|| {
                    // features are already comma separated, we still need to make them space separated
                    features.iter().flat_map(|f| f.split(' ').map(|s| s.to_string())).collect()
                }),
                hidden_features: (!hidden_features.is_empty()).then(|| {
                    // features are already comma separated, we still need to make them space separated
                    hidden_features
                        .iter()
                        .flat_map(|f| f.split(' ').map(|s| s.to_string()))
                        .collect()
                }),
                all_features: all_features.then_some(true),
                no_default_features: no_default_features.then_some(true),
                lib: target_selection.lib.then_some(true),
                bin: target_selection.bin.clone().map(|bin| match bin {
                    Some(name) => BoolOrString::String(name),
                    None => BoolOrString::Bool(true),
                }),
                toolchain: toolchain.clone(),
                target: target.clone(),
                target_dir: target_dir.clone(),
                readme_path: readme_path.clone(),
            },
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

    /// Shrink headings by this amount [default: 1]
    ///
    /// Shrinks headings when inserting documentation into the readme by
    /// the given amount. This increases the heading level (the amount of `#`).
    #[arg(global = true, long, value_name = "AMOUNT")]
    shrink_headings: Option<i8>,

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
    #[arg(global = true, help_heading = heading::MESSAGE_OPTIONS, short, long, action = clap::ArgAction::Count)]
    verbose: u8,

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

    /// Space or comma separated list of features to hide from the documentation
    #[arg(global = true, help_heading = heading::FEATURE_SELECTION, long, value_delimiter = ',', value_name = "FEATURES")]
    hidden_features: Vec<String>,

    #[command(flatten)]
    target_selection: TargetSelection,

    /// Which rustup toolchain to use when invoking rustdoc [default: "nightly-2025-12-05"]
    ///
    /// The default value is a toolchain that is known to be compatible with
    /// this version of `cargo-insert-docs`.
    ///
    /// WARNING: `cargo-insert-docs` does not consider updating the default nightly toolchain
    /// or the supported rustdoc json version a breaking change. So if you set a custom toolchain
    /// you should use a pinned version of `cargo-insert-docs`.
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

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}
