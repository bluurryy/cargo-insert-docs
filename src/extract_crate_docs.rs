mod resolver;

use std::path::PathBuf;

use cargo_metadata::Metadata;
use color_eyre::eyre::{OptionExt as _, Report, Result, bail};
use pulldown_cmark::LinkType;
use rustdoc_types::Crate;
use tracing::warn;

use crate::{
    Context, markdown, read_to_string,
    rustdoc_json::{self, CommandOutput},
    string_replacer::StringReplacer,
};

use resolver::{Resolver, ResolverOptions};

pub fn extract(cx: &Context) -> Result<String> {
    let path = generate_rustdoc_json(cx)?;
    let json = read_to_string(&path)?;
    let krate = rustdoc_json::parse(&json)?;

    extract_docs(ExtractDocsOptions {
        krate: &krate,
        metadata: &cx.metadata,
        on_not_found: &mut |link, cause| warn!(%cause, %link, "failed to resolve doc link"),
        link_to_latest: cx.cfg.link_to_latest,
    })
}

fn generate_rustdoc_json(cx: &Context) -> Result<PathBuf> {
    let command_output = if cx.args.cli.quiet {
        CommandOutput::Ignore
    } else if cx.args.cli.quiet_cargo {
        CommandOutput::Collect
    } else {
        CommandOutput::Inherit
    };

    if matches!(command_output, CommandOutput::Inherit) {
        // the command invocation will write directly to the terminal
        // setting this flag here will make the log insert a newline
        // before the next log message
        cx.log.foreign_write_incoming();
    }

    let (output, path) = rustdoc_json::generate(
        &cx.metadata,
        cx.package,
        cx.target,
        rustdoc_json::Options {
            toolchain: Some(&cx.cfg.toolchain),
            all_features: cx.cfg.all_features,
            no_default_features: cx.cfg.no_default_features,
            features: &mut cx.enabled_features.iter().map(|s| &**s),
            manifest_path: Some(cx.package.manifest_path.as_std_path()),
            target: cx.cfg.target.as_deref(),
            target_dir: cx.cfg.target_dir.as_deref(),
            quiet: cx.args.cli.quiet,
            document_private_items: cx.cfg.document_private_items,
            output: command_output,
            no_deps: cx.cfg.no_deps,
        },
    )?;

    if !output.status.success() {
        if command_output == CommandOutput::Collect {
            // write an empty line to separate our messages from the invoked command
            cx.log.foreign_write_incoming();
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }

        let see = if command_output != CommandOutput::Ignore { " (see stderr above)" } else { "" };

        bail!("Failed to build rustdoc JSON{see}");
    }

    Ok(path)
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

    let mut new_docs = StringReplacer::new(&docs);

    for link in markdown::links(&docs).into_iter().rev() {
        let markdown::Link { span, link_type, dest_url, title, id: _, content_span } = link;

        if !matches!(
            link_type,
            LinkType::Inline
                | LinkType::ReferenceUnknown
                | LinkType::CollapsedUnknown
                | LinkType::ShortcutUnknown
        ) {
            // we only handle inline links and unknown references
            continue;
        }

        let Some(&item_id) = root.links.get(&*dest_url) else {
            // rustdoc has no item for this url
            // the link could be dead because of conditional compilation
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

        new_docs.replace(span, replace_with);
    }

    let new_docs = new_docs.finish();
    let new_docs = markdown::clean_code_blocks(&new_docs);
    let new_docs = markdown::shrink_headings(&new_docs);

    let new_docs = markdown::rewrite_link_definition_urls(&new_docs, |url| {
        let Some(&item_id) = root.links.get(url) else {
            // not an intra doc link
            return None;
        };

        let url = match resolver.item_url(item_id) {
            Ok(ok) => ok,
            Err(err) => {
                on_not_found(url, err);
                return None;
            }
        };

        Some(url)
    });

    Ok(new_docs)
}
