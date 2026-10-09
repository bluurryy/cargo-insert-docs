mod resolver;
mod rewrite_markdown;

use cargo_metadata::Metadata;
use color_eyre::eyre::{OptionExt as _, Report, Result};
use rustdoc_types::Crate;
use tracing::warn;

use crate::{
    PackageContext,
    extract_crate_docs::rewrite_markdown::{RewriteMarkdownOptions, rewrite_markdown},
    package_context::CliContext,
    pretty_log::PrettyLog,
    rustdoc,
};

use resolver::{Resolver, ResolverOptions};

pub fn extract(log: &PrettyLog, cli: &CliContext, pkg: &PackageContext) -> Result<String> {
    extract_docs(ExtractDocsOptions {
        krate: &rustdoc::generate(log, cli, pkg)?,
        metadata: &pkg.metadata,
        on_not_found: &mut |link, cause| warn!(%cause, %link, "failed to resolve doc link"),
        link_to_latest: pkg.link_to_latest,
        shrink_headings: pkg.shrink_headings,
        skip_doc_links: &pkg.skip_doc_links,
    })
}

struct ExtractDocsOptions<'a> {
    krate: &'a Crate,
    metadata: &'a Metadata,
    on_not_found: &'a mut dyn FnMut(&str, Report),
    link_to_latest: bool,
    shrink_headings: i8,
    skip_doc_links: &'a [String],
}

fn extract_docs(
    ExtractDocsOptions {
        krate,
        metadata,
        on_not_found,
        link_to_latest,
        shrink_headings,
        skip_doc_links,
    }: ExtractDocsOptions,
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
            let path = match resolver.item_path(item_id) {
                Ok(ok) => ok,
                Err(err) => {
                    on_not_found(&url, err);
                    return (url, None);
                }
            };

            let is_skipped = skip_doc_links.iter().any(|b| {
                let skip_path = b.split("::").collect::<Vec<_>>();

                let path =
                    path.iter().rev().take(skip_path.len()).map(|p| p.name).collect::<Vec<_>>();

                path == skip_path
            });

            if is_skipped {
                return (url, None);
            }

            let mut new_url = resolver.item_url_from_path(&path);

            if let Some(hash) = url.find("#") {
                new_url.push_str(&url[hash..]);
            }

            (url, Some(new_url))
        })
        .collect::<Vec<_>>();

    Ok(rewrite_markdown(docs, &RewriteMarkdownOptions { shrink_headings, links }))
}
