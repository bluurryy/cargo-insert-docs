//! https://github.com/matklad/cargo-xtask

mod compare_links;
mod test_registry;
mod util;

use std::env;

use clap::Parser;
use color_eyre::eyre::bail;

use util::{OK, Result, cmd, println, re, read, write};

use crate::{
    test_registry::TestRegistry,
    util::{AnsiStripExt, cargo_insert_docs, expect_file, print_error},
};

#[derive(Parser)]
struct Args {
    /// Filter the tests by name; filtering is done by a `contains` check
    filter: Option<String>,
}

fn main() -> Result {
    color_eyre::install()?;
    util::init()?;
    let args = Args::parse();

    let mut reg = TestRegistry::default();

    reg.add("cargo-tests", || {
        let out = cmd!("cargo test --color always -- --color always")
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

        let style = anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)))
            .bold();

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
    });

    reg.add("check_test-crate", || cargo_insert_docs!("--check -p test-crate").run());

    reg.add("check_test-document-features", || {
        cargo_insert_docs!("--check -p test-document-features crate-into-readme").run()
    });

    reg.add("check_example-crate", || cargo_insert_docs!("--check -p example-crate").run());

    reg.add("check_test-bin", || cargo_insert_docs!("--check -p test-bin crate-into-readme").run());

    reg.add("check_workspace", || {
        cargo_insert_docs!(
            "--check --workspace",
            "--exclude test-crate",
            "--exclude cargo-insert-docs",
            "--exclude test-bin-lib",
            "--exclude test-bin-lib-both-active",
            "--exclude xtask",
            "--exclude test-crate-dep",
            "--exclude test-document-private-items",
            "crate-into-readme"
        )
        .run()
    });

    reg.add("test-custom-build-target", || {
        cargo_insert_docs!("crate-into-readme --check")
            .current_dir("tests/test-custom-build-target")
            .run()
    });

    reg.add("test-custom-build-target-multiple", || {
        cargo_insert_docs!("crate-into-readme --check --target wasm32-unknown-unknown")
            .current_dir("tests/test-custom-build-target-multiple")
            .run()
    });

    reg.add("recurse", || {
        fn test(feature: &str) -> Result {
            let out = cargo_insert_docs!("-p test-crate -F", feature, "--allow-dirty")
                .unchecked()
                .stderr()?;

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
    });

    reg.add("test-config", || {
        let out = cargo_insert_docs!("--manifest-path tests/test-config/Cargo.toml --print-config")
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
    });

    // FIXME: It should be an error to not specify a --bin when there are multiple
    //        (rustdoc doesn't care, but lets rather be explicit, like `cargo run`)

    reg.add("test-bin-multiple_main", || {
        cargo_insert_docs!("--check --crate-section-name main --bin test-bin-multiple")
            .current_dir("tests/test-bin-multiple")
            .run()
    });

    reg.add("test-bin-multiple_foo", || {
        cargo_insert_docs!("--check --crate-section-name foo --bin foo")
            .current_dir("tests/test-bin-multiple")
            .run()
    });

    reg.add("test-bin-multiple_bar", || {
        cargo_insert_docs!("--check --crate-section-name bar --bin bar")
            .current_dir("tests/test-bin-multiple")
            .run()
    });

    // For the `test-bin-lib_*` tests we need to choose a different target dir for lib and bin,
    // otherwise we can get stale documentation from the other target.

    reg.add("test-bin-lib_none", || {
        cargo_insert_docs!(
            "--check --target-dir tests/test-bin-lib/target/foo/lib --crate-section-name",
            Verbatim("lib documentation")
        )
        .current_dir("tests/test-bin-lib")
        .run()
    });

    reg.add("test-bin-lib_lib", || {
        cargo_insert_docs!(
            "--check --target-dir tests/test-bin-lib/target/foo/lib --lib --crate-section-name",
            Verbatim("lib documentation")
        )
        .current_dir("tests/test-bin-lib")
        .run()
    });
    reg.add("test-bin-lib_bin", || {
        cargo_insert_docs!(
            "--check --target-dir tests/test-bin-lib/target/foo/bin --bin --crate-section-name",
            Verbatim("bin documentation")
        )
        .current_dir("tests/test-bin-lib")
        .run()
    });
    reg.add("test-bin-lib_both", || {
        cargo_insert_docs!("--check --target-dir tests/test-bin-lib/target/foo/lib --lib --bin")
            .current_dir("tests/test-bin-lib")
            .expect_error_containing("cannot be used with")
    });

    reg.add("expect_bin-lib-both-active", || {
        cargo_insert_docs!("-p test-bin-lib-both-active --check")
            .expect_error_containing("choose one or the other")
    });

    reg.add("compare-links-with-html", || {
        // run cargo-insert-docs
        let stderr =
            cmd!("cargo run -q -- --check -p test-crate --quiet-cargo").stderr()?.strip_ansi();
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
                *href =
                    re!(r#"https:\/\/docs\.rs\/[^\/]+\/[^\/]+\/"#).replace(href, "../").to_string();
            }

            let diff = compare_links::diff(&html_links, &md_links);
            expect_file("tests/test-crate/links.diff", &diff)?;
        }

        OK
    });

    reg.add("test-document-private-items", || {
        cargo_insert_docs!(
            "crate-into-readme -p test-document-private-items --document-private-items --check"
        )
        .run()
    });

    // regression test for https://github.com/bluurryy/cargo-insert-docs/issues/33
    reg.add("test-custom-build-dir", || {
        let dir = "tests/test-custom-build-dir";

        cmd!("cargo clean").current_dir(dir).run()?;
        cargo_insert_docs!("--check --target-dir target/first").current_dir(dir).run()?;
        cargo_insert_docs!("--check --target-dir target/second").current_dir(dir).run()?;

        OK
    });

    reg.run(args.filter.as_deref());

    OK
}
