#[cfg(test)]
mod tests;

use std::{collections::HashSet, fmt::Write};

use color_eyre::eyre::{Result, bail};

pub fn extract(toml: &str, feature_label: &str) -> Result<String> {
    Ok(format(&parse(toml)?, feature_label))
}

type FeatureDocs = Vec<FeatureDocEntry>;

#[derive(Debug)]
enum FeatureDocEntry {
    Other { docs: String },
    Feature { name: String, docs: String, is_default: bool },
}

fn parse(toml: &str) -> Result<FeatureDocs> {
    let doc = toml_edit::ImDocument::parse(toml)?;

    let Some(features) = doc.get("features") else {
        return Ok(vec![]);
    };

    let Some(features) = features.as_table_like() else {
        return Ok(vec![]);
    };

    let mut defaults = HashSet::new();

    if let Some(item) = features.get("default")
        && let Some(array) = item.as_array()
    {
        for value in array.iter() {
            if let Some(str) = value.as_str() {
                defaults.insert(str);
            }
        }
    }

    let mut vec = vec![];

    for (key, _) in features.get_values() {
        let key = key[0];
        let name = key.get();

        if name == "default" {
            continue;
        }

        let decor = key.leaf_decor();

        let prefix = match decor.prefix() {
            Some(raw_string) => {
                let span =
                    raw_string.span().expect("`toml_edit` should return a span for `ImDocument`s");
                &doc.raw()[span]
            }
            None => "",
        };

        let mut other_docs = String::new();
        let mut feature_docs = String::new();

        for line in prefix.lines() {
            if let Some(other_comment) = comment_line(line, "#!")? {
                other_docs.push_str(other_comment);
                other_docs.push('\n');
            }

            if let Some(feature_comment) = comment_line(line, "##")? {
                feature_docs.push_str(feature_comment);
                feature_docs.push('\n');
            }
        }

        if !other_docs.is_empty() {
            vec.push(FeatureDocEntry::Other { docs: other_docs });
        }

        vec.push(FeatureDocEntry::Feature {
            name: name.to_string(),
            docs: feature_docs,
            is_default: defaults.contains(name),
        });
    }

    Ok(vec)
}

fn comment_line<'a>(line: &'a str, prefix: &str) -> Result<Option<&'a str>> {
    let Some(comment) = line.strip_prefix(prefix) else {
        return Ok(None);
    };

    comment_strip_space_for_non_empty_lines(comment).map(Some)
}

fn comment_strip_space_for_non_empty_lines(line: &str) -> Result<&str> {
    if line.chars().all(char::is_whitespace) {
        return Ok(line);
    }

    match line.strip_prefix(' ') {
        Some(line) => Ok(line),
        None => {
            // TODO: use miette errors to point to where the error is
            // and provide a help section that explains that this is to
            // prevent problems with indentation
            bail!("a non-empty feature docs comment line must start with a space")
        }
    }
}

fn format(docs: &FeatureDocs, feature_label: &str) -> String {
    let mut out = String::new();

    for doc in docs {
        match doc {
            FeatureDocEntry::Other { docs } => {
                let start_pad = if out.is_empty() { "" } else { "\n" };
                writeln!(out, "{start_pad}{docs}").unwrap();
            }
            FeatureDocEntry::Feature { name, docs, is_default } => {
                let label = feature_label.replace("{feature}", name);
                let default = if *is_default { " *(enabled by default)*" } else { "" };

                write!(out, "- {label}{default}").unwrap();

                if docs.is_empty() {
                    out.push('\n');
                } else {
                    // non-empty docs always end in a newline
                    for (i, line) in docs.lines().enumerate() {
                        // either add the em dash or indentation
                        out.push_str(if i == 0 { " — " } else { "  " });
                        out.push_str(line);
                        out.push('\n');
                    }
                };
            }
        }
    }

    out
}
