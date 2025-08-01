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
    InBetween { docs: String },
    Feature { name: String, docs: String, is_default: bool },
}

fn parse(toml: &str) -> Result<FeatureDocs> {
    let doc = toml_edit::Document::parse(toml)?;

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
            Some(raw_string) => match (raw_string.as_str(), raw_string.span()) {
                (Some(string), _) => string,
                (None, Some(span)) => &doc.raw()[span],
                (None, None) => "",
            },
            None => "",
        };

        let mut in_between_docs = String::new();
        let mut feature_docs = String::new();

        for line in prefix.lines() {
            if let Some(in_between_comment) = comment_line(line, "#!")? {
                in_between_docs.push_str(in_between_comment);
                in_between_docs.push('\n');
            }

            if let Some(feature_comment) = comment_line(line, "##")? {
                feature_docs.push_str(feature_comment);
                feature_docs.push('\n');
            }
        }

        if !in_between_docs.is_empty() {
            vec.push(FeatureDocEntry::InBetween { docs: in_between_docs });
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

    comment_line_unprefixed(comment).map(Some)
}

fn comment_line_unprefixed(mut line: &str) -> Result<&str> {
    // only full whitespace lines are allowed to not start with a space
    if !line.chars().all(char::is_whitespace) {
        line = match line.strip_prefix(' ') {
            Some(line) => line,
            None => {
                // use errors spans to point to where the error is
                // and provide a help section that explains that this is to
                // prevent problems with indentation
                bail!("a non-empty feature docs comment line must start with a space")
            }
        }
    }

    // we already trim the end when inserting into the crate docs but we might as well do it here too
    line = line.trim_end();

    Ok(line)
}

fn format(docs: &FeatureDocs, feature_label: &str) -> String {
    let mut out = String::new();

    for doc in docs {
        match doc {
            FeatureDocEntry::InBetween { docs } => {
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
