use similar::ChangeTag;
use tl::NodeHandle;

pub fn extract_links_from_md(md: &str) -> Vec<(String, String)> {
    let html = markdown::to_html_with_options(md, &markdown::Options::gfm())
        .expect("failed to parse markdown");

    let dom = tl::parse(&html, tl::ParserOptions::new()).expect("failed to parse html");

    extract_links_from_html_query(dom.parser(), dom.query_selector("a").unwrap())
}

pub fn extract_links_from_html(html: &str) -> Vec<(String, String)> {
    let dom =
        tl::parse(html, tl::ParserOptions::new().track_classes()).expect("failed to parse html");

    let doc = dom
        .query_selector(".docblock")
        .expect("invalid query selector")
        .next()
        .expect("can't find docblock")
        .get(dom.parser())
        .expect("invalid node")
        .as_tag()
        .unwrap();

    extract_links_from_html_query(dom.parser(), doc.query_selector(dom.parser(), "a").unwrap())
}

fn extract_links_from_html_query(
    parser: &tl::Parser,
    iter: impl Iterator<Item = NodeHandle>,
) -> Vec<(String, String)> {
    let mut links: Vec<(String, String)> = Vec::new();

    for node in iter {
        let a = node.get(parser).unwrap().as_tag().unwrap();

        if a.attributes().is_class_member("doc-anchor") {
            continue;
        }

        if a.attributes().is_class_member("tooltip") {
            continue;
        }

        let href = a.attributes().get("href").unwrap().unwrap().as_utf8_str().to_string();
        let html = a.inner_html(parser);

        links.push((html, href));
    }

    links
}

pub fn diff(expected: &[(String, String)], actual: &[(String, String)]) -> String {
    let expected = format(expected);
    let actual = format(actual);

    let diff = similar::TextDiff::from_lines(expected.as_str(), actual.as_str());
    let mut out = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };

        out.push_str(sign);
        out.push_str(change.as_str().unwrap());
    }

    out
}

fn format(links: &[(String, String)]) -> String {
    let mut out = String::new();

    for (html, href) in links {
        if !out.is_empty() {
            out.push('\n');
        }

        out.push_str("html: ");
        out.push_str(html);
        out.push('\n');

        out.push_str("href: ");
        out.push_str(href);
        out.push('\n');
    }

    out
}
