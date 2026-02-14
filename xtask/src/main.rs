//! https://github.com/matklad/cargo-xtask

mod compare_links;
mod util;

use std::env;

use clap::{CommandFactory, Parser, Subcommand};
use color_eyre::eyre::bail;

use util::{OK, Result, cmd, eprintln, println, re, read, write};

use crate::util::AnsiStripExt;

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
    Check,
    CheckRecurse,
    CheckConfig,
    CheckBinLib,
    CheckTestCrate,
}

fn main() -> Result {
    color_eyre::install()?;
    let args = Args::parse();

    let Some(command) = args.command else {
        Args::command().print_help()?;
        return OK;
    };

    util::init()?;

    match command {
        Command::Ci => ci(),
        Command::Test => test(),
        Command::Check => check_simple(),
        Command::CheckRecurse => check_recurse(),
        Command::CheckConfig => check_config(),
        Command::CheckBinLib => check_bin_lib_stderr(),
        Command::CheckTestCrate => check_test_crate(),
    }
}

fn ci() -> Result {
    test()?;
    check_simple()?;
    check_recurse()?;
    check_config()?;
    check_bin_lib_stderr()?;
    check_test_crate()?;
    OK
}

fn test() -> Result {
    let out = cmd!("cargo test --color always -- --color always")
        .unchecked()
        .inherit_and_capture()
        .stdout()?
        .strip_ansi();

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
        let out = cmd!(
            "cargo test",
            "--package cargo-insert-docs",
            "--bin cargo-insert-docs --all-features --",
            test,
            "--color always",
            "--exact",
            "--show-output",
            "--ignored"
        )
        .ignore_stderr()
        .stdout()?;

        let re = re!(r"(?m)(?<all>^test (?<name>.*)? \.\.\. (?<result>.*)$)");
        for c in re.captures_iter(&out) {
            let all = c?.name("all").unwrap().as_str();
            println!("{all}");
        }
    }

    OK
}

fn check_simple() -> Result {
    cmd!("cargo run -- --check -p test-crate").output()?;
    cmd!("cargo run -- --check -p test-document-features crate-into-readme").output()?;
    cmd!("cargo run -- --check -p example-crate").output()?;
    cmd!("cargo run -- --check -p test-bin crate-into-readme").output()?;
    cmd!(
        "cargo run -- --check --workspace",
        "--exclude test-crate",
        "--exclude cargo-insert-docs",
        "--exclude test-bin-lib",
        "--exclude xtask",
        "--exclude test-crate-dep",
        "crate-into-readme"
    )
    .output()?;
    OK
}

fn check_recurse() -> Result {
    fn test(feature: &str) -> Result {
        let out =
            cmd!("cargo run -- -p test-crate -F", feature, "--allow-dirty").unchecked().stderr()?;

        println!("{out}");

        if !out.contains("recursed too deep while resolving item paths") {
            println!("{out}");
            bail!("recurse test failed");
        }

        OK
    }

    test("recurse")?;
    test("recurse-glob")?;
    OK
}

fn check_config() -> Result {
    let out = cmd!("cargo run -- --manifest-path tests/test-config/Cargo.toml --print-config")
        .stdout()?;

    if env::var("UPDATE_EXPECT").as_deref() == Ok("1") {
        write("tests/test-config/print-config.toml", &out)?;
        return OK;
    }

    let expected = read("tests/test-config/print-config.toml").unwrap_or_default();

    if out != expected {
        print_error("EXPECT TEST FAILED");
        bail!("test-config failed");
    }

    OK
}

fn check_bin_lib_stderr() -> Result {
    let out = cmd!("cargo run -- -p test-bin-lib --allow-dirty").unchecked().stderr()?;

    if !out.contains("choose one or the other") {
        print_error("EXPECTED A DIFFERENT ERROR");
        bail!("test-bin-lib failed");
    }

    OK
}

fn check_test_crate() -> Result {
    // run cargo-insert-docs
    let stderr = cmd!("cargo run -q -- --check -p test-crate --quiet-cargo").stderr()?.strip_ansi();
    expect_file("tests/test-crate/stderr.txt", &stderr)?;

    // create html
    cmd!("cargo +nightly doc -p test-crate").run()?;

    // diff links
    {
        let html = read("target/doc/test_crate/index.html")?;
        let mut html_links = compare_links::extract_links_from_html(&html);

        let md = read("tests/test-crate/MEREAD.md")?;
        let mut md_links = compare_links::extract_links_from_md(&md).split_off(3);

        for (html, href) in &mut html_links {
            *html = html.replace("…", "...");
            *href = href.replace("/nightly/", "/");
        }

        for (_html, href) in &mut md_links {
            // replace this crate
            *href = href.replace("https://docs.rs/test-crate/0.0.0/test_crate/", "");

            // replace foreign crate links
            *href = re!(r#"https:\/\/docs\.rs\/[^\/]+\/[^\/]+\/"#).replace(href, "../").to_string();
        }

        let diff = compare_links::diff(&html_links, &md_links);
        expect_file("tests/test-crate/links.diff", &diff)?;
    }

    OK
}

fn print_error(message: &str) {
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))).bold();
    eprintln!("{style}{message}{style:#}");
}

fn expect_file(path: &str, content: &str) -> Result {
    let new = content;
    let old = read(path).unwrap_or_default();

    if new != old {
        if is_update_expect() {
            let style = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
                .bold();

            eprintln!("{style}EXPECTED FILE CONTENT CHANGED{style:#}");
            eprintln!("{path}");

            write(path, new)?;
        } else {
            print_error("EXPECTED FILE CONTENT MISMATCH");
            eprintln!("{path}");

            bail!("expected file content mismatch")
        }
    }

    OK
}

fn is_update_expect() -> bool {
    env::var("UPDATE_EXPECT").as_deref() == Ok("1")
}
