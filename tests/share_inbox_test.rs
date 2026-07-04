// Share-inbox envelope parsing and drain routing: text -> note, URL -> note
// with title + clickable source card, file -> note + attachment via the
// existing import pipeline, malformed envelopes dropped without wedging the
// queue. Title fetching stays OFF here (drain_with) so tests run offline.

use flowflow::application::share_inbox::{
    drain_with, extract_article, extract_title, parse_entry, url_domain,
    url_sources_json, SharedEntry,
};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

#[test]
fn parses_the_three_envelope_kinds() {
    assert_eq!(
        parse_entry(r#"{"kind":"text","text":"une idée"}"#),
        Some(SharedEntry::Text("une idée".into()))
    );
    assert_eq!(
        parse_entry(r#"{"kind":"url","url":"https://a.be"}"#),
        Some(SharedEntry::Url("https://a.be".into()))
    );
    assert_eq!(
        parse_entry(r#"{"kind":"file","file":"x-doc.pdf","name":"doc.pdf"}"#),
        Some(SharedEntry::File {
            stored: "x-doc.pdf".into(),
            display_name: "doc.pdf".into()
        })
    );
}

#[test]
fn rejects_malformed_envelopes() {
    assert_eq!(parse_entry("not json"), None);
    assert_eq!(parse_entry(r#"{"kind":"alien"}"#), None);
    assert_eq!(parse_entry(r#"{"kind":"text","text":"  "}"#), None);
    assert_eq!(parse_entry(r#"{"kind":"file"}"#), None);
}

#[test]
fn extracts_and_decodes_html_titles() {
    assert_eq!(
        extract_title("<html><head><title>Rust &amp; iOS</title></head>"),
        Some("Rust & iOS".to_string())
    );
    assert_eq!(
        extract_title(r#"<TITLE lang="fr">  Café &#39;noir&#39;  </TITLE>"#),
        Some("Café 'noir'".to_string())
    );
    assert_eq!(extract_title("<title></title>"), None);
    assert_eq!(extract_title("no title here"), None);
}

#[test]
fn url_helpers_shape_domain_and_source_card() {
    assert_eq!(
        url_domain("https://fr.wikipedia.org/wiki/Rust"),
        "fr.wikipedia.org"
    );
    assert_eq!(url_domain("weird"), "weird");
    let json = url_sources_json("Rust", "https://fr.wikipedia.org/wiki/Rust");
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v[0]["title"], "Rust");
    assert_eq!(v[0]["url"], "https://fr.wikipedia.org/wiki/Rust");
}

#[test]
fn extracts_readable_article_and_drops_boilerplate() {
    let para = "Le protocole WebSocket permet une communication \
bidirectionnelle persistante entre un client et un serveur au-dessus d'une \
seule connexion TCP, contrairement au cycle requête-réponse de HTTP. "
        .repeat(3);
    let html = format!(
        "<html><head><title>WebSocket — Wikipédia</title></head><body>\
<nav><a href=\"/\">Accueil</a><a href=\"/aide\">Aide</a></nav>\
<article><h1>WebSocket</h1><p>{para}</p><p>{para}</p></article>\
<footer>Licence CC</footer></body></html>"
    );
    let (title, text) =
        extract_article(&html, "https://fr.wikipedia.org/wiki/WebSocket")
            .expect("article extracted");
    assert!(title.contains("WebSocket"));
    assert!(text.contains("bidirectionnelle"));
    assert!(!text.contains("Accueil"));

    // A page with no real content falls back to None (plain link note).
    assert_eq!(extract_article("<html><body>rien</body></html>", "u"), None);
}

#[tokio::test]
async fn drain_creates_notes_and_attachments_and_empties_inbox() {
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    let inbox = dir.path().join("shared-inbox");
    std::fs::create_dir_all(&inbox).unwrap();

    std::fs::write(
        inbox.join("a.json"),
        r#"{"kind":"text","text":"pensée partagée"}"#,
    )
    .unwrap();
    std::fs::write(
        inbox.join("b.json"),
        r#"{"kind":"url","url":"https://exemple.be/article"}"#,
    )
    .unwrap();
    std::fs::write(inbox.join("doc-1.txt"), "contenu du document partagé")
        .unwrap();
    std::fs::write(
        inbox.join("c.json"),
        r#"{"kind":"file","file":"doc-1.txt","name":"notes.txt"}"#,
    )
    .unwrap();
    std::fs::write(inbox.join("bad.json"), "garbage").unwrap();

    let n = drain_with(&db, &inbox, false).await;
    assert_eq!(n, 3);

    let notes = db.list_notes().unwrap();
    assert_eq!(notes.len(), 3);

    // URL note: domain title + clickable source card payload.
    let url_note = notes
        .iter()
        .find(|x| x.title.as_deref() == Some("exemple.be"))
        .expect("url note");
    assert_eq!(url_note.content, "https://exemple.be/article");
    let sources = url_note.sources_json.as_deref().expect("sources set");
    assert!(sources.contains("https://exemple.be/article"));

    let carrier = notes
        .iter()
        .find(|x| x.title.as_deref() == Some("notes.txt"))
        .expect("carrier note");
    let atts = db.list_attachments_for_note(&carrier.id).unwrap();
    assert_eq!(atts.len(), 1);
    assert_eq!(atts[0].content_text, "contenu du document partagé");

    // Inbox fully drained, including the payload file and the bad envelope.
    assert_eq!(std::fs::read_dir(&inbox).unwrap().count(), 0);
}

#[tokio::test]
async fn drain_skips_unparseable_files_but_removes_them() {
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    let inbox = dir.path().join("shared-inbox");
    std::fs::create_dir_all(&inbox).unwrap();

    std::fs::write(inbox.join("img-1.png"), [0u8; 10]).unwrap();
    std::fs::write(
        inbox.join("a.json"),
        r#"{"kind":"file","file":"img-1.png","name":"photo.png"}"#,
    )
    .unwrap();

    assert_eq!(drain_with(&db, &inbox, false).await, 0);
    assert!(db.list_notes().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(&inbox).unwrap().count(), 0);
}
