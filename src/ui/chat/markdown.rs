use pulldown_cmark::{
    html, CowStr, Event, LinkType, Options, Parser, Tag, TagEnd,
};
use regex::Regex;
use std::sync::LazyLock;

// The frame carries the border; the inner box scrolls and fades its hidden
// edge (tailwind.css, .prose .table-scroll).
const TABLE_OPEN: &str =
    r#"<div class="table-frame"><div class="table-scroll">"#;
const TABLE_CLOSE: &str = "</div></div>";

static BARE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s<>"]+"#).unwrap());

pub fn md_to_html(md: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, events(md).into_iter());
    out
}

fn events(md: &str) -> Vec<Event<'_>> {
    // GFM tables/strikethrough/tasklists are off by default in pulldown-cmark, so a `| a | b |`
    // table from the agent rendered as raw piped text. Enable them.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let mut out = Vec::new();
    // pulldown-cmark may split one run of text in several events; a URL
    // must be read whole before it is linked.
    let mut text = String::new();
    let mut in_link = false;
    let mut in_code = false;
    for event in Parser::new_ext(md, opts) {
        if let Event::Text(t) = &event {
            if !in_link && !in_code {
                text.push_str(t);
                continue;
            }
        }
        linkify(&mut text, &mut out);
        match event {
            // Agent, Hermes and shared-note text is untrusted and lands in innerHTML:
            // raw HTML shows as text, and a link only keeps a harmless scheme.
            Event::Html(raw) | Event::InlineHtml(raw) => {
                out.push(Event::Text(raw))
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                in_link = true;
                let dest_url = if is_safe_href(&dest_url) {
                    dest_url
                } else {
                    "".into()
                };
                out.push(Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }));
            }
            Event::Start(Tag::Image { .. }) => {
                in_link = true;
                out.push(event);
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                in_link = false;
                out.push(event);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                out.push(event);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                out.push(event);
            }
            Event::Start(Tag::Table(_)) => {
                out.push(Event::Html(TABLE_OPEN.into()));
                out.push(event);
            }
            Event::End(TagEnd::Table) => {
                out.push(event);
                out.push(Event::Html(TABLE_CLOSE.into()));
            }
            other => out.push(other),
        }
    }
    linkify(&mut text, &mut out);
    out
}

/// Emits the pending text, with every bare http(s) address as a link.
fn linkify(text: &mut String, out: &mut Vec<Event<'_>>) {
    if text.is_empty() {
        return;
    }
    let mut rest = 0;
    for m in BARE_URL.find_iter(text) {
        let url = trim_url(m.as_str());
        if m.start() > rest {
            out.push(Event::Text(text[rest..m.start()].to_string().into()));
        }
        let url: CowStr = url.to_string().into();
        out.push(Event::Start(Tag::Link {
            link_type: LinkType::Autolink,
            dest_url: url.clone(),
            title: "".into(),
            id: "".into(),
        }));
        out.push(Event::Text(url.clone()));
        out.push(Event::End(TagEnd::Link));
        rest = m.start() + url.len();
    }
    if rest < text.len() {
        out.push(Event::Text(text[rest..].to_string().into()));
    }
    text.clear();
}

/// Sentence punctuation after an address is not part of it, nor a `)`
/// closing a parenthesis opened before the address.
fn trim_url(url: &str) -> &str {
    let mut url = url;
    loop {
        let trimmed =
            url.trim_end_matches(['.', ',', ';', ':', '!', '?', '\'', '"']);
        let trimmed = if trimmed.ends_with(')')
            && trimmed.matches(')').count() > trimmed.matches('(').count()
        {
            &trimmed[..trimmed.len() - 1]
        } else {
            trimmed
        };
        if trimmed.len() == url.len() {
            return url;
        }
        url = trimmed;
    }
}

fn is_safe_href(href: &str) -> bool {
    let Some((scheme, _)) = href.split_once(':') else {
        return true;
    };
    scheme.contains(['/', '?', '#'])
        || matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto" | "tel" | "flowflow"
        )
}
