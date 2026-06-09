use flowflow::db::chunk_repo::{blob_to_vector, vector_to_blob, ChunkRecord};
use flowflow::db::Database;
use flowflow::models::{NewAttachment, NewTextNote};
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

fn rec(id: &str, owner: &str, kind: &str, idx: i32, dim: usize) -> ChunkRecord {
    ChunkRecord {
        id: id.to_string(),
        owner_id: owner.to_string(),
        owner_kind: kind.to_string(),
        chunk_index: idx,
        vector: vec![0.25_f32; dim],
        content_hash: "hash".to_string(),
        chunk_text: "text".to_string(),
        title: "title".to_string(),
        tags: "[]".to_string(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    }
}

#[test]
fn blob_roundtrips_f32_little_endian() {
    let v = vec![1.0_f32, -2.5, 3.14159, 0.0, f32::MIN, f32::MAX];
    let blob = vector_to_blob(&v);
    assert_eq!(blob.len(), v.len() * 4);
    assert_eq!(blob_to_vector(&blob), v);
}

#[test]
fn replace_chunks_writes_n_blobs() {
    let (db, _dir) = open_test_db();
    let recs = vec![
        rec("note:N:0", "N", "note", 0, 1536),
        rec("note:N:1", "N", "note", 1, 1536),
    ];
    db.replace_chunks("N", "note", &recs).expect("replace");

    assert_eq!(db.count_chunks_for_owner("N", "note").unwrap(), 2);
    let back = db.chunks_for_owner("N", "note").unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].vector.len(), 1536);
    assert_eq!(back[0].chunk_index, 0);
    assert_eq!(back[1].chunk_index, 1);
}

#[test]
fn replace_chunks_is_atomic_no_orphan() {
    let (db, _dir) = open_test_db();
    let three = vec![
        rec("note:N:0", "N", "note", 0, 8),
        rec("note:N:1", "N", "note", 1, 8),
        rec("note:N:2", "N", "note", 2, 8),
    ];
    db.replace_chunks("N", "note", &three).expect("first");
    assert_eq!(db.count_chunks_for_owner("N", "note").unwrap(), 3);

    let one = vec![rec("note:N:0", "N", "note", 0, 8)];
    db.replace_chunks("N", "note", &one)
        .expect("re-embed fewer");
    assert_eq!(
        db.count_chunks_for_owner("N", "note").unwrap(),
        1,
        "re-embed with fewer chunks must leave no orphan"
    );
}

#[test]
fn note_and_attachment_chunks_are_independent_in_sqlite() {
    let (db, _dir) = open_test_db();
    db.replace_chunks("N", "note", &[rec("note:N:0", "N", "note", 0, 4)])
        .expect("note");
    db.replace_chunks(
        "A",
        "attachment",
        &[rec("att:A:0", "A", "attachment", 0, 4)],
    )
    .expect("attachment");

    assert_eq!(db.count_chunks_for_owner("N", "note").unwrap(), 1);
    assert_eq!(db.count_chunks_for_owner("A", "attachment").unwrap(), 1);

    db.replace_chunks("N", "note", &[rec("note:N:0", "N", "note", 0, 4)])
        .expect("re-embed note");
    assert_eq!(
        db.count_chunks_for_owner("A", "attachment").unwrap(),
        1,
        "re-embedding the note must not touch the attachment chunks"
    );
}

#[test]
fn deleting_note_purges_note_and_attachment_chunks() {
    let (db, _dir) = open_test_db();
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("n".to_string()),
            content: "x".repeat(60),
            tags: vec![],
        })
        .expect("note");
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note.id.clone(),
            filename: "f.txt".to_string(),
            content_text: "y".repeat(60),
        })
        .expect("attachment");

    db.replace_chunks(
        &note.id,
        "note",
        &[rec(&format!("note:{}:0", note.id), &note.id, "note", 0, 4)],
    )
    .expect("note chunks");
    db.replace_chunks(
        &att.id,
        "attachment",
        &[rec(
            &format!("att:{}:0", att.id),
            &att.id,
            "attachment",
            0,
            4,
        )],
    )
    .expect("att chunks");

    db.delete_note(&note.id).expect("delete note");

    assert_eq!(
        db.count_chunks_for_owner(&note.id, "note").unwrap(),
        0,
        "note chunks purged on delete"
    );
    assert_eq!(
        db.count_chunks_for_owner(&att.id, "attachment").unwrap(),
        0,
        "attachment chunks purged on parent-note delete"
    );
}

#[test]
fn deleting_attachment_purges_only_its_chunks() {
    let (db, _dir) = open_test_db();
    let note = db
        .create_text_note(&NewTextNote {
            title: Some("n".to_string()),
            content: "x".repeat(60),
            tags: vec![],
        })
        .expect("note");
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note.id.clone(),
            filename: "f.txt".to_string(),
            content_text: "y".repeat(60),
        })
        .expect("attachment");

    db.replace_chunks(
        &note.id,
        "note",
        &[rec(&format!("note:{}:0", note.id), &note.id, "note", 0, 4)],
    )
    .expect("note chunks");
    db.replace_chunks(
        &att.id,
        "attachment",
        &[rec(
            &format!("att:{}:0", att.id),
            &att.id,
            "attachment",
            0,
            4,
        )],
    )
    .expect("att chunks");

    db.delete_attachment(&att.id).expect("delete attachment");

    assert_eq!(
        db.count_chunks_for_owner(&att.id, "attachment").unwrap(),
        0,
        "attachment chunks purged on delete"
    );
    assert_eq!(
        db.count_chunks_for_owner(&note.id, "note").unwrap(),
        1,
        "note chunks untouched by attachment delete"
    );
}
