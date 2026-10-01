use std::process::{Command, Stdio};

use anstream::ColorChoice;
use color_eyre::eyre::Result;
use color_eyre::eyre::{Context, bail};
use tracing::error_span;

use crate::{
    config::is_lib_like,
    package_context::{CliContext, PackageContext},
    pretty_log::PrettyLog,
};

/// A command helper to easily add additional arguments.
pub fn command() -> CargoRustdocCommand {
    Default::default()
}

#[derive(Default)]
pub struct CargoRustdocCommand {
    additional_cargo_args: Vec<String>,
}

impl CargoRustdocCommand {
    pub fn cargo_arg(&mut self, arg: &str) -> &mut Self {
        self.additional_cargo_args.push(arg.into());
        self
    }

    pub fn run(&self, log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<Vec<u8>> {
        let mut cmd = Command::new("cargo");

        if cli.quiet_cargo {
            cmd.stderr(Stdio::null());
        } else {
            log.foreign_write_incoming();
            cmd.stderr(Stdio::inherit());
        }

        cmd.arg(format!("+{}", pkg.toolchain));

        cmd.args([
            "rustdoc",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
            "--message-format",
            "json-render-diagnostics",
        ]);

        for z_flag in &cli.z_flags {
            cmd.arg(format!("-Z{z_flag}"));
        }

        if is_lib_like(&pkg.cargo_target) {
            cmd.arg("--lib");
        } else if pkg.cargo_target.is_bin() {
            cmd.arg("--bin").arg(&pkg.cargo_target.name);
        } else {
            bail!("target must be lib or bin")
        }

        if cli.quiet {
            cmd.arg("--quiet");
        }

        cmd.arg("--color").arg(match cli.color {
            ColorChoice::Auto => "auto",
            ColorChoice::Always => "always",
            ColorChoice::Never | ColorChoice::AlwaysAnsi => "never",
        });

        cmd.arg("--manifest-path");
        cmd.arg(&pkg.manifest_path);

        if let Some(target) = pkg.target.as_deref() {
            cmd.arg("--target");
            cmd.arg(target);
        }

        cmd.arg("--target-dir");
        cmd.arg(&pkg.target_dir);

        if pkg.all_features {
            cmd.arg("--all-features");
        }

        if pkg.no_default_features {
            cmd.arg("--no-default-features");
        }

        for feature in &pkg.features {
            cmd.arg("--features").arg(feature);
        }

        cmd.arg("--package").arg(&pkg.id.repr);
        cmd.args(&self.additional_cargo_args);

        // The rest are flags only for rustdoc
        cmd.arg("--");

        if pkg.document_private_items {
            cmd.arg("--document-private-items");
        }

        let output = cmd.output().wrap_err("failed to spawn cargo")?;

        if !output.status.success() {
            let _span =
                output.status.code().map(|status_code| error_span!("", %status_code).entered());

            if cli.quiet {
                bail!("you shouldn't be able to see this :/");
            } else {
                bail!("Failed to build rustdoc JSON (see stderr above)");
            }
        }

        Ok(output.stdout)
    }
}
