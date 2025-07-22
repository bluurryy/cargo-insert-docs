mod resolver;

use cargo_metadata::Metadata;
use color_eyre::eyre::{OptionExt as _, Report, Result, bail};
use rustdoc_types::Crate;
use tracing::warn;

use crate::{
    Context, markdown, read_to_string,
    rustdoc_json::{self, CommandOutput},
};

use resolver::{Resolver, ResolverOptions};

pub fn extract(cx: &Context) -> Result<String> {
    generate_rustdoc_json(cx)?;
    let path = rustdoc_json::path(&cx.metadata, cx.package.target)?;
    let json = read_to_string(path.as_std_path())?;
    let krate = rustdoc_json::parse(&json)?;

    extract_docs(ExtractDocsOptions {
        krate: &krate,
        metadata: &cx.metadata,
        on_not_found: &mut |link, cause| warn!(%cause, %link, "failed to resolve doc link"),
        link_to_latest: cx.args.link_to_latest,
    })
}

fn generate_rustdoc_json(cx: &Context) -> Result<()> {
    let command_output = if cx.args.quiet {
        CommandOutput::Ignore
    } else if cx.args.quiet_cargo {
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

    let output = rustdoc_json::generate(
        &cx.metadata,
        &cx.package.id,
        cx.package.target,
        rustdoc_json::Options {
            toolchain: Some(&cx.args.toolchain),
            all_features: cx.args.all_features,
            no_default_features: cx.args.no_default_features,
            features: &mut cx.package.enabled_features.iter().map(|s| &**s),
            manifest_path: cx.args.manifest_path.as_deref(),
            target: cx.args.target.as_deref(),
            quiet: cx.args.quiet,
            document_private_items: cx.args.document_private_items,
            output: command_output,
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

    Ok(())
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
