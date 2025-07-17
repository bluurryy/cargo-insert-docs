mod resolver;

use std::fs;

use cargo_metadata::Metadata;
use color_eyre::eyre::{Context as _, OptionExt as _, Report, Result, bail};
use rustdoc_json::Color;
use rustdoc_types::Crate;
use serde::Deserialize;
use tracing::warn;

use crate::{Context, markdown};

use resolver::{Resolver, ResolverOptions};

pub fn extract(cx: &Context) -> Result<String> {
    let json = create_rustdoc_json(cx)?;
    let krate = parse_rustdoc_json(&json)?;

    extract_docs(ExtractDocsOptions {
        krate: &krate,
        metadata: &cx.metadata,
        on_not_found: &mut |link, cause| warn!(%cause, %link, "failed to resolve doc link"),
        link_to_latest: cx.args.link_to_latest,
    })
}

fn create_rustdoc_json(cx: &Context) -> Result<String> {
    let mut builder = rustdoc_json::Builder::default()
        .toolchain(&cx.args.toolchain)
        .manifest_path(&cx.args.manifest_path)
        .all_features(cx.args.all_features)
        .no_default_features(cx.args.no_default_features)
        .features(&cx.package.enabled_features)
        .document_private_items(cx.args.document_private_items)
        .color(Color::Always);

    if cx.package.is_explicit {
        builder = builder.package(&cx.metadata[&cx.package.id].name);
    }

    if let Some(target) = cx.args.target.as_ref() {
        builder = builder.target(target.to_string());
    }

    if cx.args.quiet {
        builder = builder.quiet(true).silent(true);
    }

    if !cx.args.quiet_cargo {
        // the command invocation will write to stdout
        // setting this flag here will make the log insert a newline
        // before the next log message
        cx.log.foreign_write_incoming();
    }

    let json_path = if cx.args.quiet_cargo && !cx.args.quiet {
        // if we silence cargo we still want to print stderr if an error occured
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        match builder.build_with_captured_output(&mut stdout, &mut stderr) {
            Ok(ok) => ok,
            Err(err) => {
                // write an empty line to separate our messages from the invoked command
                cx.log.foreign_write_incoming();
                eprint!("{}", String::from_utf8_lossy(&stderr));
                return Err(err.into());
            }
        }
    } else {
        builder.build()?
    };

    fs::read_to_string(json_path).context("failed to read generated rustdoc json")
}

fn parse_rustdoc_json(rustdoc_json: &str) -> Result<Crate, Report> {
    #[derive(Deserialize)]
    struct CrateWithJustTheFormatVersion {
        format_version: u32,
    }

    let krate: CrateWithJustTheFormatVersion =
        serde_json::from_str(rustdoc_json).context("failed to parse generated rustdoc json")?;

    if krate.format_version != rustdoc_types::FORMAT_VERSION {
        let expected = rustdoc_types::FORMAT_VERSION;
        let actual = krate.format_version;
        let what_to_do = if actual > expected {
            "update `cargo-insert-docs` or use an older nightly toolchain"
        } else {
            "upgrade your nightly toolchain"
        };

        bail!(
            "`cargo-insert-docs` requires rustdoc json format version {expected} \
            but rustdoc produced version {actual}\n\
            {what_to_do} to be able to use this tool"
        );
    }

    serde_json::from_str(rustdoc_json).context("failed to parse generated rustdoc json")
}

struct ExtractDocsOptions<'a> {
    krate: &'a Crate,
    metadata: &'a Metadata,
    on_not_found: &'a mut dyn FnMut(&str, Report),
    link_to_latest: bool,
}

fn extract_docs(
    ExtractDocsOptions { krate, metadata, on_not_found, link_to_latest }: ExtractDocsOptions,
) -> Result<String, Report> {
    let root = krate.index.get(&krate.root).ok_or_eyre("crate index has no root")?;
    let docs = root.docs.as_deref().unwrap_or("").to_string();

    let resolver_options = ResolverOptions { link_to_latest };
    let resolver = Resolver::new(krate, metadata, &resolver_options)?;

    let mut new_docs = docs.clone();

    for link in markdown::links(&docs).into_iter().rev() {
        let markdown::Link { span, link_type: _, dest_url, title, id: _, content_span } = link;

        let Some(&item_id) = root.links.get(&*dest_url) else {
            // rustdoc has no item for this url
            // the link could just not be rustdoc related like `https://www.rust-lang.org/` or
            // the link is dead because some feature is not enabled or code is cfg'd out
            // we keep such links as they are
            continue;
        };

        let url = match resolver.item_url(item_id) {
            Ok(ok) => Some(ok),
            Err(err) => {
                on_not_found(&dest_url, err);
                None
            }
        };

        let content = &docs[content_span.clone().unwrap_or(0..0)];

        let replace_with = match url {
            Some(mut url) => {
                // You can link to sections within an item's documentation by writing `[Vec](Vec#guarantees)`.
                if let Some(hash) = dest_url.find("#") {
                    url.push_str(&dest_url[hash..]);
                }

                use std::fmt::Write;
                let mut s = String::new();

                write!(s, "[{content}]({url}").unwrap();

                if !title.is_empty() {
                    write!(s, " \"{title}\"").unwrap();
                }

                write!(s, ")").unwrap();
                s
            }
            None => content.to_string(),
        };

        new_docs.replace_range(span, &replace_with);
    }

    let new_docs = markdown::clean_code_blocks(&new_docs);
    let new_docs = markdown::shrink_headings(&new_docs);

    Ok(new_docs)
}
