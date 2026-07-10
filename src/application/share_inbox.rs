use crate::application::embed::{embed_attachment, embed_note};
use crate::application::note_persistence::create_note;
use crate::domain::NewAttachment;
use crate::infrastructure::persistence::Database;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 20 * 1024 * 1024;
const PARSEABLE: &[&str] = &["txt", "md", "csv", "pdf", "docx"];

/// One queued share, parsed from the extension's JSON envelope.
#[derive(Debug, PartialEq)]
pub enum SharedEntry {
    Text(String),
    Url(String),
    File {
        stored: String,
        display_name: String,
    },
}

/// Parse one inbox envelope; None for anything malformed (skipped + removed,
/// a poisoned entry must not wedge the queue forever).
pub fn parse_entry(json: &str) -> Option<SharedEntry> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    match v.get("kind")?.as_str()? {
        "text" => {
            let t = v.get("text")?.as_str()?.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(SharedEntry::Text(t))
            }
        }
        "url" => {
            let u = v.get("url")?.as_str()?.trim().to_string();
            if u.is_empty() {
                None
            } else {
                Some(SharedEntry::Url(u))
            }
        }
        "file" => Some(SharedEntry::File {
            stored: v.get("file")?.as_str()?.to_string(),
            display_name: v
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("document")
                .to_string(),
        }),
        _ => None,
    }
}

fn parseable(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| PARSEABLE.iter().any(|p| e.eq_ignore_ascii_case(p)))
        .unwrap_or(false)
}

fn import_shared_file(
    db: &Database,
    inbox: &Path,
    stored: &str,
    display_name: &str,
) -> bool {
    let path = inbox.join(stored);
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size == 0 || size > MAX_FILE_SIZE || !parseable(display_name) {
        eprintln!("[share-inbox] skip {display_name} ({size} bytes)");
        return false;
    }
    let is_pdf = display_name.to_ascii_lowercase().ends_with(".pdf");
    let content = if is_pdf {
        crate::infrastructure::platform::pdf::extract(&path)
    } else {
        crate::infrastructure::platform::parsers::read_file_as_text(&path)
    };
    let text = match content {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[share-inbox] parse {display_name}: {e}");
            return false;
        }
    };
    // Same shape as the paperclip import: a carrier note + the document as
    // an attachment with its own chunked embeddings.
    let Some(note) = create_note(db, display_name, "", vec![], None, None)
    else {
        return false;
    };
    let new_att = NewAttachment {
        note_id: note.id.clone(),
        filename: display_name.to_string(),
        content_text: text,
    };
    match db.create_attachment(&new_att) {
        Ok(att) => {
            embed_attachment(
                att.id.clone(),
                note.id.clone(),
                att.filename.clone(),
                att.content_text.clone(),
            );
            true
        }
        Err(e) => {
            eprintln!("[share-inbox] attachment {display_name}: {e}");
            false
        }
    }
}

fn import_text_note(db: &Database, content: &str) -> bool {
    let Some(note) = create_note(db, "", content, vec![], None, None) else {
        return false;
    };
    embed_note(
        note.id,
        note.title.unwrap_or_default(),
        note.content,
        note.tags,
        note.created_at,
    );
    true
}

/// `<title>` of an HTML document, entity-decoded just enough for display.
/// None when the page has no usable title.
pub fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = html[start..].find('>')? + start + 1;
    let close = lower[open_end..].find("</title>")? + open_end;
    let raw = html[open_end..close].trim();
    if raw.is_empty() {
        return None;
    }
    let decoded = raw
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    Some(crate::application::ai::char_prefix(decoded.trim(), 120))
}

pub fn url_domain(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

async fn fetch_page_body(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .user_agent("FlowFlow/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    match client.get(url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => Some(body),
            Err(e) => {
                eprintln!("[share-inbox] fetch body {url}: {e}");
                None
            }
        },
        Err(e) => {
            eprintln!("[share-inbox] fetch {url}: {e}");
            None
        }
    }
}

/// Readable article (title + text, boilerplate stripped) out of a fetched
/// page. None when extraction yields nothing substantial - the caller then
/// falls back to a plain link note.
pub fn extract_article(html: &str, url: &str) -> Option<(String, String)> {
    let mut r = dom_smoothie::Readability::new(html, Some(url), None).ok()?;
    let article = r.parse().ok()?;
    let text = article.text_content.trim().to_string();
    if text.chars().count() < 200 {
        return None;
    }
    let title = article.title.trim().to_string();
    let title = if title.is_empty() {
        extract_title(html)?
    } else {
        crate::application::ai::char_prefix(&title, 120)
    };
    Some((title, text))
}

/// The clickable source card (NoteWebSources) reads ChatSource-shaped JSON.
pub fn url_sources_json(title: &str, url: &str) -> String {
    serde_json::json!([{
        "note_id": "",
        "title": title,
        "chunk_text": "",
        "distance": 0.0,
        "created_at": "",
        "url": url,
    }])
    .to_string()
}

async fn import_url_note(db: &Database, url: &str, fetch_title: bool) -> bool {
    let body = if fetch_title {
        fetch_page_body(url).await
    } else {
        None
    };
    let article = body.as_deref().and_then(|b| extract_article(b, url));

    let note_title = article
        .as_ref()
        .map(|(t, _)| t.clone())
        .or_else(|| body.as_deref().and_then(extract_title))
        .unwrap_or_else(|| url_domain(url));

    let Some(note) = create_note(db, &note_title, url, vec![], None, None)
    else {
        return false;
    };
    let _ = db
        .set_note_sources(&note.id, Some(&url_sources_json(&note_title, url)));

    // The readable article rides the attachment pipeline: its own chunked
    // embeddings make the page content answerable in the RAG chat.
    if let Some((_, text)) = article {
        let new_att = NewAttachment {
            note_id: note.id.clone(),
            filename: format!("{note_title}.md"),
            content_text: text,
        };
        if let Ok(att) = db.create_attachment(&new_att) {
            embed_attachment(
                att.id.clone(),
                note.id.clone(),
                att.filename.clone(),
                att.content_text.clone(),
            );
        }
    }
    embed_note(
        note.id,
        note_title,
        note.content,
        note.tags,
        note.created_at,
    );
    true
}

/// Drain the app-group inbox the Share extension fills: every envelope
/// becomes a note (text/URL with page title + clickable source card) or a
/// note + attachment (file, reusing the document-import pipeline). Envelopes
/// and payload files are always removed so a bad entry cannot wedge the
/// queue. Returns how many notes landed.
pub async fn drain(db: &Database, inbox: &PathBuf) -> usize {
    drain_with(db, inbox, true).await
}

/// `fetch_titles: false` keeps tests offline (title falls back to the domain).
pub async fn drain_with(
    db: &Database,
    inbox: &PathBuf,
    fetch_titles: bool,
) -> usize {
    let Ok(entries) = std::fs::read_dir(inbox) else {
        return 0;
    };
    let mut envelopes: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    envelopes.sort();
    let mut imported = 0;
    for envelope in envelopes {
        let parsed = std::fs::read_to_string(&envelope)
            .ok()
            .and_then(|s| parse_entry(&s));
        match parsed {
            Some(SharedEntry::Text(t)) => {
                if import_text_note(db, &t) {
                    imported += 1;
                }
            }
            Some(SharedEntry::Url(u)) => {
                if import_url_note(db, &u, fetch_titles).await {
                    imported += 1;
                }
            }
            Some(SharedEntry::File {
                stored,
                display_name,
            }) => {
                if import_shared_file(db, inbox, &stored, &display_name) {
                    imported += 1;
                }
                let _ = std::fs::remove_file(inbox.join(&stored));
            }
            None => {
                eprintln!("[share-inbox] malformed envelope, dropping");
            }
        }
        let _ = std::fs::remove_file(&envelope);
    }
    if imported > 0 {
        eprintln!("[share-inbox] imported {imported} shared item(s)");
    }
    imported
}
