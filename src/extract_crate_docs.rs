mod resolver;
mod rewrite_markdown;

use std::path::PathBuf;

use cargo_metadata::Metadata;
use color_eyre::eyre::{OptionExt as _, Report, Result};
use rustdoc_types::Crate;
use tracing::warn;

use crate::{
    PackageContext,
    extract_crate_docs::rewrite_markdown::{RewriteMarkdownOptions, rewrite_markdown},
    read_to_string, rustdoc_json,
};

use resolver::{Resolver, ResolverOptions};

pub fn extract(cx: &PackageContext) -> Result<String> {
    let path = generate_rustdoc_json(cx)?;
    let json = read_to_string(&path)?;
    let krate = rustdoc_json::parse(&json, &cx.cfg.toolchain)?;

    extract_docs(ExtractDocsOptions {
        krate: &krate,
        metadata: &cx.metadata,
        on_not_found: &mut |link, cause| warn!(%cause, %link, "failed to resolve doc link"),
        link_to_latest: cx.cfg.link_to_latest,
        shrink_headings: cx.cfg.shrink_headings,
    })
}

fn generate_rustdoc_json(cx: &PackageContext) -> Result<PathBuf> {
    let target_dir = match cx.cfg.target_dir.clone() {
        Some(target_dir) => target_dir,
        None => cx.metadata.target_directory.join("insert-docs").into_std_path_buf(),
    };

    let path = rustdoc_json::generate(rustdoc_json::Options {
        package: cx.package,
        package_target: cx.target,
        toolchain: Some(&cx.cfg.toolchain),
        all_features: cx.cfg.all_features,
        no_default_features: cx.cfg.no_default_features,
        features: &mut cx.enabled_features.iter().map(|s| &**s),
        manifest_path: Some(cx.package.manifest_path.as_std_path()),
        target: cx.cfg.target.as_deref(),
        target_dir: Some(&target_dir),
        quiet: cx.cli.cfg.quiet,
        quiet_cargo: cx.cli.cfg.quiet_cargo,
        document_private_items: cx.cfg.document_private_items,
        no_deps: cx.cfg.no_deps,
        log: cx.log.clone(),
    })?;

    Ok(path)
}

struct ExtractDocsOptions<'a> {
    krate: &'a Crate,
    metadata: &'a Metadata,
    on_not_found: &'a mut dyn FnMut(&str, Report),
    link_to_latest: bool,
    shrink_headings: i8,
}

fn extract_docs(
    ExtractDocsOptions { krate, metadata, on_not_found, link_to_latest, shrink_headings }: ExtractDocsOptions,
) -> Result<String, Report> {
    let root = krate.index.get(&krate.root).ok_or_eyre("crate index has no root")?;
    let docs = root.docs.as_deref().unwrap_or("");

    let resolver_options = ResolverOptions { link_to_latest };
    let resolver = Resolver::new(krate, metadata, &resolver_options)?;

    let mut links = root.links.iter().map(|(k, &v)| (k.clone(), v)).collect::<Vec<_>>();
    links.sort_by(|(a, _), (b, _)| a.cmp(b));

    let links = links
        .into_iter()
        .map(|(url, item_id)| {
            let mut new_url = match resolver.item_url(item_id) {
                Ok(ok) => ok,
                Err(err) => {
                    on_not_found(&url, err);
                    return (url, None);
                }
            };

            if let Some(hash) = url.find("#") {
                new_url.push_str(&url[hash..]);
            }

            (url, Some(new_url))
        })
        .collect::<Vec<_>>();

    Ok(rewrite_markdown(docs, &RewriteMarkdownOptions { shrink_headings, links }))
}
