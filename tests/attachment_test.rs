use flowflow::db::Database;
use flowflow::models::{Attachment, NewAttachment, NewTextNote};
use std::io::Write;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

fn create_test_note(db: &Database, content: &str) -> String {
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("Test note".to_string()),
            content: content.to_string(),
            tags: vec![],
        })
        .expect("create note");
    note.id
}

#[test]
fn test_migration_v3_creates_attachments_table() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'attachments'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(exists, 1, "attachments table should exist after V3");
}

#[test]
fn test_migration_v3_attachments_columns() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let mut stmt = conn
        .prepare("PRAGMA table_info(attachments)")
        .expect("prepare pragma");
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>("name"))
        .expect("query")
        .filter_map(Result::ok)
        .collect();
    assert!(cols.contains(&"id".to_string()));
    assert!(cols.contains(&"note_id".to_string()));
    assert!(cols.contains(&"filename".to_string()));
    assert!(cols.contains(&"content_text".to_string()));
    assert!(cols.contains(&"imported_at".to_string()));
}

#[test]
fn test_migration_v3_index_exists() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_attachments_note'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(exists, 1, "idx_attachments_note index should exist");
}

#[test]
fn test_migration_v3_recorded() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let max_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(max_version, 4, "max migration version should be 4");
}

#[test]
fn test_create_attachment_returns_valid_attachment() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "Parent note content");

    let attached = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "doc.txt".to_string(),
            content_text: "Hello world".to_string(),
        })
        .expect("create attachment");

    assert!(!attached.id.is_empty());
    assert_eq!(attached.note_id, note_id);
    assert_eq!(attached.filename, "doc.txt");
    assert_eq!(attached.content_text, "Hello world");
    assert!(!attached.imported_at.is_empty());
}

#[test]
fn test_get_attachment_returns_some_when_exists() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "n");
    let created = db
        .create_attachment(&NewAttachment {
            note_id,
            filename: "f.md".to_string(),
            content_text: "body".to_string(),
        })
        .expect("create");
    let fetched: Option<Attachment> =
        db.get_attachment(&created.id).expect("get");
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, created.id);
}

#[test]
fn test_get_attachment_returns_none_when_missing() {
    let (db, _dir) = open_test_db();
    let fetched = db.get_attachment("nonexistent-id").expect("get");
    assert!(fetched.is_none());
}

#[test]
fn test_list_attachments_for_note_returns_correct_list() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "parent");
    let other_id = create_test_note(&db, "other");

    db.create_attachment(&NewAttachment {
        note_id: note_id.clone(),
        filename: "a.txt".to_string(),
        content_text: "alpha".to_string(),
    })
    .unwrap();
    db.create_attachment(&NewAttachment {
        note_id: note_id.clone(),
        filename: "b.txt".to_string(),
        content_text: "bravo".to_string(),
    })
    .unwrap();
    db.create_attachment(&NewAttachment {
        note_id: other_id.clone(),
        filename: "c.txt".to_string(),
        content_text: "charlie".to_string(),
    })
    .unwrap();

    let list = db
        .list_attachments_for_note(&note_id)
        .expect("list for note");
    assert_eq!(list.len(), 2);
    let filenames: Vec<&str> =
        list.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"a.txt"));
    assert!(filenames.contains(&"b.txt"));

    let other_list = db
        .list_attachments_for_note(&other_id)
        .expect("list for other");
    assert_eq!(other_list.len(), 1);
    assert_eq!(other_list[0].filename, "c.txt");
}

#[test]
fn test_list_attachments_for_note_empty() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "no attachments");
    let list = db.list_attachments_for_note(&note_id).expect("list");
    assert!(list.is_empty());
}

#[test]
fn test_delete_attachment_removes_single() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "p");
    let a = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "a.txt".to_string(),
            content_text: "alpha".to_string(),
        })
        .unwrap();
    let b = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "b.txt".to_string(),
            content_text: "bravo".to_string(),
        })
        .unwrap();

    db.delete_attachment(&a.id).expect("delete a");

    assert!(db.get_attachment(&a.id).unwrap().is_none());
    assert!(db.get_attachment(&b.id).unwrap().is_some());

    let remaining = db.list_attachments_for_note(&note_id).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, b.id);
}

#[test]
fn test_delete_attachments_for_note_removes_all() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "p");
    for n in 0..3 {
        db.create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: format!("f{n}.txt"),
            content_text: format!("c{n}"),
        })
        .unwrap();
    }
    assert_eq!(db.list_attachments_for_note(&note_id).unwrap().len(), 3);
    db.delete_attachments_for_note(&note_id)
        .expect("delete all");
    assert!(db.list_attachments_for_note(&note_id).unwrap().is_empty());
}

#[test]
fn test_cascade_delete_note_removes_attachments() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "doomed");
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "doomed.txt".to_string(),
            content_text: "bye".to_string(),
        })
        .unwrap();

    assert!(db.get_attachment(&att.id).unwrap().is_some());

    db.delete_note(&note_id).expect("delete note");

    assert!(
        db.get_attachment(&att.id).unwrap().is_none(),
        "attachment should be CASCADE deleted with parent note"
    );
}

#[test]
fn test_attachment_id_is_uuid_v4() {
    let (db, _dir) = open_test_db();
    let note_id = create_test_note(&db, "p");
    let att = db
        .create_attachment(&NewAttachment {
            note_id,
            filename: "f.txt".to_string(),
            content_text: "c".to_string(),
        })
        .unwrap();
    let parsed = uuid::Uuid::parse_str(&att.id);
    assert!(parsed.is_ok(), "attachment id should be a valid UUID");
}

#[test]
fn test_attachment_struct_serde_roundtrip() {
    let original = Attachment {
        id: "att-1".to_string(),
        note_id: "note-1".to_string(),
        filename: "f.pdf".to_string(),
        content_text: "Texte avec accents éàù".to_string(),
        imported_at: "2026-05-11T10:00:00.000Z".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: Attachment =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, restored);
}

#[test]
fn test_attachment_struct_clone() {
    let a = Attachment {
        id: "1".to_string(),
        note_id: "n".to_string(),
        filename: "f".to_string(),
        content_text: "c".to_string(),
        imported_at: "t".to_string(),
    };
    let cloned = a.clone();
    assert_eq!(a, cloned);
}

fn write_minimal_docx(path: &std::path::Path, paragraphs: &[&str]) {
    let file = std::fs::File::create(path).expect("create docx");
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("word/document.xml", opts).unwrap();
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#,
    );
    for p in paragraphs {
        body.push_str(&format!(
            "<w:p><w:r><w:t>{}</w:t></w:r></w:p>",
            xml_escape(p)
        ));
    }
    body.push_str("</w:body></w:document>");
    zip.write_all(body.as_bytes()).unwrap();

    zip.finish().expect("finish zip");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[test]
fn test_read_file_as_text_docx_basic() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sample.docx");
    write_minimal_docx(&path, &["Hello world", "Second paragraph"]);

    let file = std::fs::File::open(&path).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("zip archive");
    let mut doc = archive.by_name("word/document.xml").expect("document.xml");
    let mut xml = String::new();
    use std::io::Read;
    doc.read_to_string(&mut xml).unwrap();

    assert!(xml.contains("Hello world"));
    assert!(xml.contains("Second paragraph"));
}

#[test]
fn test_read_file_as_text_docx_with_accents() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("accents.docx");
    write_minimal_docx(&path, &["Réunion système éàù"]);

    let file = std::fs::File::open(&path).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("zip");
    let mut doc = archive.by_name("word/document.xml").unwrap();
    let mut xml = String::new();
    use std::io::Read;
    doc.read_to_string(&mut xml).unwrap();

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut buf = Vec::new();
    let mut out = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            quick_xml::events::Event::Start(ref e) => {
                if e.name().as_ref() == b"w:t" {
                    in_text = true;
                }
            }
            quick_xml::events::Event::End(ref e) => {
                if e.name().as_ref() == b"w:t" {
                    in_text = false;
                }
            }
            quick_xml::events::Event::Text(t) => {
                if in_text {
                    out.push_str(&t.unescape().unwrap());
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    assert!(out.contains("Réunion"));
    assert!(out.contains("système"));
    assert!(out.contains("éàù"));
}

#[test]
fn test_pdf_extract_crate_compiles_and_runs() {
    let result = pdf_extract::extract_text_from_mem(b"not a valid pdf");
    assert!(
        result.is_err(),
        "non-PDF bytes should produce an extraction error"
    );
}
