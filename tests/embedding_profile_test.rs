use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use flowflow::application::constants::{EMBEDDING_DIMS, EMBEDDING_PROFILE};
use flowflow::application::embed::purge_stale_embeddings;
use flowflow::domain::note::NewTextNote;
use flowflow::infrastructure::persistence::chunk_repo::{
    vector_to_blob, ChunkRecord,
};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{peers, protocol};
use std::net::TcpListener;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();

fn open_db() -> (Arc<Database>, tempfile::TempDir) {
    let dir = tempdir().expect("db tempdir");
    let db =
        Database::open_at(dir.path().join("flowflow.db")).expect("open_at");
    (Arc::new(db), dir)
}

fn chunk(id: &str, owner_id: &str, embed_profile: &str) -> ChunkRecord {
    ChunkRecord {
        id: id.to_string(),
        owner_id: owner_id.to_string(),
        owner_kind: "note".to_string(),
        chunk_index: 0,
        embed_profile: embed_profile.to_string(),
        vector: vec![0.5; EMBEDDING_DIMS],
        content_hash: "hash".to_string(),
        chunk_text: "chunk text".to_string(),
        title: "title".to_string(),
        tags: "[]".to_string(),
        created_at: "2026-09-02T00:00:00.000Z".to_string(),
    }
}

fn make_note(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("profile note".to_string()),
        content: "content long enough to embed".to_string(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn pair(db_a: &Arc<Database>, db_b: &Arc<Database>) -> String {
    ADVERTISE.call_once(|| {
        std::env::set_var("FLOWFLOW_SYNC_ADVERTISE_ADDR", "127.0.0.1");
    });
    let host = peers::start_pairing_host(db_a.clone()).expect("host");
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode uri");
    let host_device_id = peers::join_pairing(db_b, &uri).expect("join");
    for _ in 0..100 {
        if host.status() != peers::PairingStatus::Waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(matches!(host.status(), peers::PairingStatus::Paired { .. }));
    host_device_id
}

fn sync_once(
    db_host: &Arc<Database>,
    db_client: &Arc<Database>,
    host_device_id: &str,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    protocol::sync_with_peer(db_client, host_device_id, "127.0.0.1", port, 50)
        .expect("client sync");
    server.join().expect("join").expect("server sync");
}

#[test]
fn migration_and_repository_preserve_current_profile() {
    let (db, _dir) = open_db();
    let note_id = make_note(&db);
    let record = chunk("note:current:0", &note_id, EMBEDDING_PROFILE);

    db.replace_chunks(&note_id, "note", &[record])
        .expect("replace chunks");

    let rows = db.chunks_for_owner(&note_id, "note").expect("chunks");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].embed_profile, EMBEDDING_PROFILE);
}

#[test]
fn purge_removes_only_foreign_profiles_and_is_idempotent() {
    let (db, _dir) = open_db();
    let current_owner = make_note(&db);
    let foreign_owner = make_note(&db);
    db.replace_chunks(
        &current_owner,
        "note",
        &[chunk("note:current:0", &current_owner, EMBEDDING_PROFILE)],
    )
    .expect("current chunk");
    db.replace_chunks(
        &foreign_owner,
        "note",
        &[chunk("note:foreign:0", &foreign_owner, EMBEDDING_PROFILE)],
    )
    .expect("foreign seed");
    db.conn()
        .execute(
            "UPDATE chunks SET embed_profile = ?1 WHERE id = ?2",
            ["legacy:other-model:512", "note:foreign:0"],
        )
        .expect("force foreign profile");

    assert_eq!(purge_stale_embeddings(&db).expect("purge"), 1);
    assert_eq!(
        db.count_chunks_for_owner(&current_owner, "note")
            .expect("current count"),
        1
    );
    assert_eq!(
        db.count_chunks_for_owner(&foreign_owner, "note")
            .expect("foreign count"),
        0
    );
    assert_eq!(purge_stale_embeddings(&db).expect("second purge"), 0);
}

#[test]
fn sync_applies_note_but_skips_foreign_profile_chunk() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();
    let host_device_id = pair(&db_a, &db_b);
    let note_id = make_note(&db_a);
    db_a.replace_chunks(
        &note_id,
        "note",
        &[chunk(
            "note:foreign-sync:0",
            &note_id,
            "legacy:other-model:512",
        )],
    )
    .expect("foreign chunk");

    sync_once(&db_a, &db_b, &host_device_id);

    assert!(db_b.get_note(&note_id).expect("note query").is_some());
    assert_eq!(
        db_b.count_chunks_for_owner(&note_id, "note")
            .expect("chunk count"),
        0
    );
}

#[test]
fn legacy_payload_without_profile_uses_current_profile() {
    let (db, _dir) = open_db();
    let vector_b64 =
        URL_SAFE_NO_PAD.encode(vector_to_blob(&vec![0.5; EMBEDDING_DIMS]));
    let chunks = serde_json::json!([{
        "id": "note:legacy:0",
        "chunk_index": 0,
        "vector_b64": vector_b64,
        "content_hash": "hash",
        "chunk_text": "legacy chunk",
        "title": "title",
        "tags": "[]",
        "created_at": "2026-09-02T00:00:00.000Z"
    }]);
    let conn = db.conn();

    protocol::apply_chunks_for_test(
        &conn,
        "legacy-owner",
        "note",
        &chunks.to_string(),
    )
    .expect("apply legacy payload");
    drop(conn);

    let rows = db
        .chunks_for_owner("legacy-owner", "note")
        .expect("legacy chunks");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].embed_profile, EMBEDDING_PROFILE);
}
