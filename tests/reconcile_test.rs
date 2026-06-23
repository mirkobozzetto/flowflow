use flowflow::application::constants::EMBEDDING_DIMS;
use flowflow::domain::{NewAttachment, NewTextNote};
use flowflow::infrastructure::persistence::chunk_repo::ChunkRecord;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::reconcile::{
    backfill_legacy_chunks, reconcile_once, reconstruct_from_blob,
};
use flowflow::infrastructure::vectordb::{Chunk, VectorStore};
use std::sync::{Mutex, MutexGuard};
use tempfile::tempdir;

// FLOWFLOW_VECTORDB_PATH is process-global; serialize tests so each one owns
// the env var + its isolated LanceDB path for its whole body, regardless of
// the harness thread count.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn vec_dims(slot: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; EMBEDDING_DIMS];
    v[slot % EMBEDDING_DIMS] = 1.0;
    v
}

fn isolate_vectordb() -> (tempfile::TempDir, MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempdir().expect("vdb tempdir");
    std::env::set_var("FLOWFLOW_VECTORDB_PATH", dir.path());
    (dir, guard)
}

fn open_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("db tempdir");
    let db =
        Database::open_at(dir.path().join("flowflow.db")).expect("open_at");
    (db, dir)
}

fn note_with_text(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("n".to_string()),
        content: "x".repeat(60),
        tags: vec![],
    })
    .expect("note")
    .id
}

fn det_record(owner_id: &str, kind: &str, id: &str, idx: i32) -> ChunkRecord {
    ChunkRecord {
        id: id.to_string(),
        owner_id: owner_id.to_string(),
        owner_kind: kind.to_string(),
        chunk_index: idx,
        vector: vec_dims(idx as usize),
        content_hash: "h".to_string(),
        chunk_text: format!("text {idx}"),
        title: "t".to_string(),
        tags: "[]".to_string(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    }
}

fn legacy_chunk(rand_id: &str, note_id: &str, idx: i32) -> Chunk {
    Chunk {
        id: rand_id.to_string(),
        note_id: note_id.to_string(),
        chunk_text: format!("legacy {idx}"),
        chunk_index: idx,
        vector: vec_dims(idx as usize),
        title: "t".to_string(),
        tags: "[]".to_string(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn reconstruct_rebuilds_lancedb_from_sqlite_blobs() {
    let (_vdb, _lock) = isolate_vectordb();
    let (db, _dir) = open_db();
    let note_id = note_with_text(&db);
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "f.txt".to_string(),
            content_text: "y".repeat(60),
        })
        .expect("attachment");

    let note_chunk_id = format!("note:{note_id}:0");
    let att_chunk_id = format!("att:{}:0", att.id);
    db.replace_chunks(
        &note_id,
        "note",
        &[det_record(&note_id, "note", &note_chunk_id, 0)],
    )
    .unwrap();
    db.replace_chunks(
        &att.id,
        "attachment",
        &[det_record(&att.id, "attachment", &att_chunk_id, 0)],
    )
    .unwrap();

    let store = VectorStore::open().await.unwrap();
    let n = reconstruct_from_blob(&db, &store).await.unwrap();
    assert_eq!(n, 2, "rebuilds note + attachment chunks");

    let ids = store.all_ids().await.unwrap();
    assert!(ids.contains(&note_chunk_id), "note chunk present: {ids:?}");
    assert!(
        ids.contains(&att_chunk_id),
        "attachment chunk present (parent resolved): {ids:?}"
    );
}

#[tokio::test]
async fn reconcile_deletes_orphans_and_adds_missing_idempotent() {
    let (_vdb, _lock) = isolate_vectordb();
    let (db, _dir) = open_db();
    let note_id = note_with_text(&db);
    let det_id = format!("note:{note_id}:0");
    db.replace_chunks(
        &note_id,
        "note",
        &[det_record(&note_id, "note", &det_id, 0)],
    )
    .unwrap();

    let store = VectorStore::open().await.unwrap();
    store
        .add_chunks(vec![legacy_chunk("rand-uuid-1", &note_id, 0)])
        .await
        .unwrap();

    reconcile_once(&db, &store).await.unwrap();
    let ids = store.all_ids().await.unwrap();
    assert_eq!(
        ids,
        vec![det_id.clone()],
        "orphan deleted, deterministic added"
    );

    reconcile_once(&db, &store).await.unwrap();
    let ids2 = store.all_ids().await.unwrap();
    assert_eq!(ids2, vec![det_id], "second pass is a no-op (idempotent)");
}

#[tokio::test]
async fn backfill_copies_vectors_then_reconcile_drops_random_ids() {
    let (_vdb, _lock) = isolate_vectordb();
    let (db, _dir) = open_db();
    let note_id = note_with_text(&db);

    let store = VectorStore::open().await.unwrap();
    store
        .add_chunks(vec![legacy_chunk("rand-legacy", &note_id, 0)])
        .await
        .unwrap();

    backfill_legacy_chunks(&db, &store).await.unwrap();
    assert_eq!(
        db.count_chunks_for_owner(&note_id, "note").unwrap(),
        1,
        "legacy LanceDB vector copied into SQLite BLOB"
    );
    let recs = db.chunks_for_owner(&note_id, "note").unwrap();
    assert_eq!(recs[0].id, format!("note:{note_id}:0"), "deterministic id");
    assert_eq!(recs[0].vector.len(), EMBEDDING_DIMS, "vector preserved");

    store
        .add_chunks(vec![legacy_chunk("rand-legacy-2", &note_id, 1)])
        .await
        .unwrap();
    backfill_legacy_chunks(&db, &store).await.unwrap();
    assert_eq!(
        db.count_chunks_for_owner(&note_id, "note").unwrap(),
        1,
        "backfill flag set: runs once"
    );

    reconcile_once(&db, &store).await.unwrap();
    let ids = store.all_ids().await.unwrap();
    assert!(
        ids.iter()
            .all(|i| i.starts_with("note:") || i.starts_with("att:")),
        "0 random-id row remaining: {ids:?}"
    );
    assert!(ids.contains(&format!("note:{note_id}:0")));
}

#[tokio::test]
async fn recovery_empty_lancedb_reconstructs_with_no_embed() {
    let (_vdb, _lock) = isolate_vectordb();
    let (db, _dir) = open_db();
    let note_id = note_with_text(&db);
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "f.txt".to_string(),
            content_text: "y".repeat(60),
        })
        .expect("attachment");

    let note_chunk_id = format!("note:{note_id}:0");
    let att_chunk_id = format!("att:{}:0", att.id);
    db.replace_chunks(
        &note_id,
        "note",
        &[det_record(&note_id, "note", &note_chunk_id, 0)],
    )
    .unwrap();
    db.replace_chunks(
        &att.id,
        "attachment",
        &[det_record(&att.id, "attachment", &att_chunk_id, 0)],
    )
    .unwrap();

    let store = VectorStore::open().await.unwrap();
    reconcile_once(&db, &store).await.unwrap();
    let ids = store.all_ids().await.unwrap();
    assert_eq!(ids.len(), 2, "empty LanceDB rebuilt from BLOB: {ids:?}");
    assert!(ids.contains(&note_chunk_id));
    assert!(ids.contains(&att_chunk_id));
}

#[tokio::test]
async fn deleted_attachment_leftovers_do_not_break_convergence() {
    let (_vdb, _lock) = isolate_vectordb();
    let (db, _dir) = open_db();
    let note_id = note_with_text(&db);
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "f.txt".to_string(),
            content_text: "y".repeat(60),
        })
        .expect("attachment");
    let att_id = att.id.clone();

    db.delete_attachment(&att_id).expect("delete attachment");

    let store = VectorStore::open().await.unwrap();
    store
        .add_chunks(vec![
            legacy_chunk("rand-note", &note_id, 0),
            legacy_chunk(&format!("att:{att_id}:0"), &note_id, 0),
        ])
        .await
        .unwrap();

    backfill_legacy_chunks(&db, &store).await.unwrap();
    assert_eq!(
        db.count_chunks_for_owner(&att_id, "attachment").unwrap(),
        0,
        "backfill must not re-introduce chunks for a deleted attachment"
    );
    db.replace_chunks(
        &att_id,
        "attachment",
        &[det_record(
            &att_id,
            "attachment",
            &format!("att:{att_id}:0"),
            0,
        )],
    )
    .unwrap();

    reconcile_once(&db, &store).await.unwrap();
    assert_eq!(
        db.count_chunks_for_owner(&att_id, "attachment").unwrap(),
        0,
        "self-heal purges SQLite chunks of a deleted owner"
    );
    let ids = store.all_ids().await.unwrap();
    assert_eq!(
        ids,
        vec![format!("note:{note_id}:0")],
        "converges to live note chunk only: {ids:?}"
    );

    reconcile_once(&db, &store).await.unwrap();
    let ids2 = store.all_ids().await.unwrap();
    assert_eq!(ids2, vec![format!("note:{note_id}:0")], "idempotent");
}
