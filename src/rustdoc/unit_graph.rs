use cargo_metadata::{PackageId, Target};
use color_eyre::eyre::{Context as _, Result, bail};
use serde::Deserialize;
use tracing::error_span;

/// Parses cargo's `--unit-graph` output.
pub fn parse(bytes: &[u8]) -> Result<UnitGraph> {
    // first, only check the version field of the unit graph so we can exit early instead of getting
    // a deserialization error
    let graph_stub: UnitGraphWithJustTheVersion =
        serde_json::from_slice(bytes).wrap_err("failed to parse cargo unit graph stub")?;

    let expected = 1;
    let actual = graph_stub.version;

    if actual != expected {
        let _span = error_span!("", help = "use a supported nightly toolchain");
        bail!("expected cargo unit graph version {expected}, got {actual}");
    }

    let graph: UnitGraph =
        serde_json::from_slice(bytes).wrap_err("failed to parse cargo unit graph")?;

    Ok(graph)
}

#[derive(Deserialize)]
struct UnitGraphWithJustTheVersion {
    version: u32,
}

#[derive(Deserialize)]
pub struct UnitGraph {
    pub roots: Vec<usize>,
    pub units: Vec<Unit>,
}

#[derive(Deserialize)]
pub struct Unit {
    pub pkg_id: PackageId,
    pub target: Target,
    pub platform: Option<String>,
    pub mode: String,
}
