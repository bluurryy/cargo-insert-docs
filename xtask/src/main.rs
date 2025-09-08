/// https://github.com/matklad/cargo-xtask
use std::path::Path;

use anstream::println;
use clap::{CommandFactory, Parser, Subcommand};
use color_eyre::eyre::{OptionExt, bail};
use xshell::{Shell, cmd};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Clone)]
enum Command {
    /// Does all tests and checks
    Ci,
    Test,
    CheckRecurse,
    CheckConfig,
    CheckBinLib,
    Check,
}

type Error = color_eyre::eyre::Report;
type Result<T = (), E = Error> = std::result::Result<T, E>;
const OK: Result = Result::Ok(());

fn main() -> Result {
    color_eyre::install()?;
    let args = Args::parse();

    let Some(command) = args.command else {
        Args::command().print_help()?;
        return OK;
    };

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().ok_or_eyre("manifest dir has no parent")?;

    let sh = Shell::new()?;
    sh.change_dir(workspace_dir);

    match command {
        Command::Ci => ci(&sh),
        Command::Test => test(&sh),
        Command::Check => check(&sh),
        Command::CheckRecurse => check_recurse(&sh),
        Command::CheckConfig => check_config(&sh),
        Command::CheckBinLib => check_bin_lib(&sh),
    }
}

fn ci(sh: &Shell) -> Result {
    test(sh)?;
    check(sh)?;
    check_recurse(sh)?;
    check_config(sh)?;
    check_bin_lib(sh)?;
    OK
}

macro_rules! re {
    ($lit:literal) => {{
        fn get() -> &'static fancy_regex::Regex {
            static REGEX: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
            REGEX.get_or_init(|| fancy_regex::Regex::new($lit).unwrap())
        }
        get()
    }};
}

fn test(sh: &Shell) -> Result {
    // TODO: tee stderr/stdout
    let out = cmd!(sh, "cargo test --color always -- --color always").ignore_status().output()?;
    println!("\nstdout: {}\n", String::from_utf8_lossy(&out.stdout));
    println!("\nstderr: {}\n", String::from_utf8_lossy(&out.stderr));

    if !out.status.success() {
        bail!("cargo test failed: {:?}", out.status);
    }

    let out = anstream::adapter::strip_str(&String::from_utf8_lossy(&out.stdout)).to_string();

    let tests_that_need_to_be_run_separately: Vec<_> =
        re!(r"(?m)^test (?<name>.*)? \.\.\. (?<result>.*)$")
            .captures_iter(&out)
            .filter_map(|c| {
                let c = c.unwrap();

                if c.name("result").unwrap().as_str()
                    == "ignored, needs to be run separately because of hooks"
                {
                    Some(c.name("name").unwrap().as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))).bold();
    println!("{style}NOW RUNNING PREVIOUSLY IGNORED TESTS{style:#}");

    for test in tests_that_need_to_be_run_separately {
        let out = cmd!(sh, "cargo test --package cargo-insert-docs --bin cargo-insert-docs --all-features -- {test} --color always --exact --show-output --ignored").ignore_stderr().read()?;
        let re = re!(r"(?m)(?<all>^test (?<name>.*)? \.\.\. (?<result>.*)$)");
        for c in re.captures_iter(&out) {
            let all = c?.name("all").unwrap().as_str();
            println!("{all}");
        }
    }

    OK
}

fn check_recurse(sh: &Shell) -> Result {
    fn test(sh: &Shell, feature: &str) -> Result {
        let out = cmd!(sh, "cargo run -- -p test-crate -F {feature} --allow-dirty")
            .ignore_status()
            .read_stderr()?;
        println!("{out}");

        if !out.contains("recursed too deep while resolving item paths") {
            println!("{out}");
            bail!("recurse test failed");
        }

        OK
    }

    test(sh, "recurse")?;
    test(sh, "recurse-glob")?;
    OK
}

fn check_config(sh: &Shell) -> Result {
    let out = cmd!(sh, "cargo run -- --manifest-path tests/test-config/Cargo.toml --print-config")
        .read()?;

    if std::env::var("UPDATE_EXPECT").as_deref() == Ok("1") {
        sh.write_file("tests/test-config/print-config.toml", &out)?;
        return OK;
    }

    let expected = sh.read_file("tests/test-config/print-config.toml").unwrap_or_default();

    if out != expected {
        print_error("EXPECT TEST FAILED");
        bail!("test-config failed");
    }

    OK
}

fn check_bin_lib(sh: &Shell) -> Result {
    let out =
        cmd!(sh, "cargo run -- -p test-bin-lib --allow-dirty").ignore_status().read_stderr()?;

    if !out.contains("choose one or the other") {
        print_error("EXPECTED A DIFFERENT ERROR");
        bail!("test-bin-lib failed");
    }

    OK
}

fn check(sh: &Shell) -> Result {
    cmd!(sh, "cargo run -- --check -p test-crate").run()?;
    cmd!(sh, "cargo run -- --check -p test-document-features crate-into-readme").run()?;
    cmd!(sh, "cargo run -- --check -p example-crate").run()?;
    cmd!(sh, "cargo run -- --check -p test-bin crate-into-readme").run()?;
    cmd!(sh, "cargo run -- --check --workspace --exclude test-crate --exclude cargo-insert-docs --exclude test-bin-lib --exclude xtask --exclude test-crate-dep crate-into-readme").run()?;
    OK
}

fn print_error(message: &str) {
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))).bold();
    eprintln!("{style}{message}{style:#}");
}
