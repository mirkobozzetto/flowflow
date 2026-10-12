use pulldown_cmark::{
    html, CodeBlockKind, CowStr, Event, LinkType, Options, Parser, Tag, TagEnd,
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
// How Hermes names a file it made (gateway/platforms/base.py,
// MEDIA_TAG_CLEANUP_RE): a quoted path, or an absolute or ~/ path that ends
// in an extension.
static MEDIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"MEDIA:\s*(?:`([^`\n]+)`|"([^"\n]+)"|'([^'\n]+)'|((?:~/|/)\S+\.[A-Za-z0-9]{1,8}))"#,
    )
    .unwrap()
});

/// A reply cut where the UI renders something richer than HTML: a code block
/// with its copy button, a file card.
#[derive(Debug, Clone, PartialEq)]
pub enum MdBlock {
    Html(String),
    Code { lang: String, code: String },
    File { name: String, kind: String },
}

pub fn md_to_html(md: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, events(md).into_iter());
    out
}

/// Only top-level blocks are cut out: a code block or a file inside a list
/// keeps its plain HTML rendering.
pub fn md_blocks(md: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut html = Vec::new();
    let mut block = Vec::new();
    let mut depth = 0usize;
    for event in events(md) {
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        if depth == 0 && block.is_empty() {
            html.push(event);
            continue;
        }
        block.push(event);
        if depth > 0 {
            continue;
        }
        let parts = top_level(std::mem::take(&mut block));
        match parts {
            Ok(events) => html.extend(events),
            Err(parts) => {
                flush_html(&mut html, &mut blocks);
                blocks.extend(parts);
            }
        }
    }
    flush_html(&mut html, &mut blocks);
    blocks
}

/// `Ok` keeps the block as HTML, `Err` replaces it by richer parts.
fn top_level(block: Vec<Event<'_>>) -> Result<Vec<Event<'_>>, Vec<MdBlock>> {
    match block.first() {
        Some(Event::Start(Tag::CodeBlock(kind))) => {
            let lang = match kind {
                CodeBlockKind::Fenced(info) => {
                    info.split_whitespace().next().unwrap_or("").to_string()
                }
                CodeBlockKind::Indented => String::new(),
            };
            let code: String = block
                .iter()
                .filter_map(|e| match e {
                    Event::Text(t) => Some(t.as_ref()),
                    _ => None,
                })
                .collect();
            let code = code.trim_end_matches('\n').to_string();
            Err(vec![MdBlock::Code { lang, code }])
        }
        Some(Event::Start(Tag::Paragraph)) => {
            let text: String = block
                .iter()
                .filter_map(|e| match e {
                    Event::Text(t) | Event::Code(t) => Some(t.as_ref()),
                    Event::SoftBreak | Event::HardBreak => Some(" "),
                    _ => None,
                })
                .collect();
            let files: Vec<MdBlock> = MEDIA
                .captures_iter(&text)
                .filter_map(|c| (1..=4).find_map(|i| c.get(i)))
                .map(|path| file(path.as_str()))
                .collect();
            if files.is_empty() {
                return Ok(block);
            }
            let rest = MEDIA.replace_all(&text, "");
            let rest = rest.trim();
            let mut parts = Vec::new();
            // A lone "." left by "MEDIA:/x.pdf." is no sentence.
            if rest.chars().any(char::is_alphanumeric) {
                let mut html = String::new();
                html::push_html(
                    &mut html,
                    [
                        Event::Start(Tag::Paragraph),
                        Event::Text(rest.into()),
                        Event::End(TagEnd::Paragraph),
                    ]
                    .into_iter(),
                );
                parts.push(MdBlock::Html(html));
            }
            parts.extend(files);
            Err(parts)
        }
        _ => Ok(block),
    }
}

fn file(path: &str) -> MdBlock {
    let name = path.trim().rsplit(['/', '\\']).next().unwrap_or(path);
    let kind = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_uppercase())
        .unwrap_or_default();
    MdBlock::File {
        name: name.to_string(),
        kind,
    }
}

fn flush_html(html: &mut Vec<Event<'_>>, blocks: &mut Vec<MdBlock>) {
    if html.is_empty() {
        return;
    }
    let mut out = String::new();
    html::push_html(&mut out, html.drain(..));
    blocks.push(MdBlock::Html(out));
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
