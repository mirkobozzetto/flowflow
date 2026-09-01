use flowflow::infrastructure::persistence::apply_migrations;
use rusqlite::Connection;

#[test]
fn failed_migration_rolls_back_data_and_version() {
    let conn = Connection::open_in_memory().expect("open database");
    conn.execute_batch(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
         INSERT INTO notes (id, body) VALUES (1, 'kept');
         CREATE TABLE _migrations (version INTEGER PRIMARY KEY);
         INSERT INTO _migrations (version) VALUES (1);",
    )
    .expect("seed database");

    let result = apply_migrations(
        &conn,
        &[(
            2,
            "DELETE FROM notes; INSERT INTO notes VALUES (2, 'lost');
              SELECT * FROM missing_table;",
        )],
    );

    assert!(result.is_err(), "migration must fail");
    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT id, body FROM notes ORDER BY id")
        .expect("prepare notes")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query notes")
        .collect::<Result<_, _>>()
        .expect("read notes");
    assert_eq!(rows, vec![(1, "kept".to_owned())]);
    let versions: Vec<i64> = conn
        .prepare("SELECT version FROM _migrations ORDER BY version")
        .expect("prepare versions")
        .query_map([], |row| row.get(0))
        .expect("query versions")
        .collect::<Result<_, _>>()
        .expect("read versions");
    assert_eq!(versions, vec![1]);
}

const PENDING_REBUILD: &str = "
    CREATE TABLE pending_transcriptions_v11 (
        note_id TEXT PRIMARY KEY,
        transcription_id TEXT,
        soniox_file_id TEXT,
        created_at TEXT NOT NULL
    );
    INSERT INTO pending_transcriptions_v11
        (note_id, transcription_id, soniox_file_id, created_at)
        SELECT note_id, transcription_id, soniox_file_id, created_at
        FROM pending_transcriptions;
    DROP TABLE pending_transcriptions;
    ALTER TABLE pending_transcriptions_v11 RENAME TO pending_transcriptions;
";

const CONVERSATION_REBUILD: &str = "
    CREATE TABLE conversation_messages_new (
        id TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL,
        content TEXT NOT NULL
    );
    INSERT INTO conversation_messages_new (id, conversation_id, content)
        SELECT id, conversation_id, content FROM conversation_messages;
    DROP TABLE conversation_messages;
    ALTER TABLE conversation_messages_new RENAME TO conversation_messages;
    CREATE INDEX IF NOT EXISTS idx_cm_conversation
        ON conversation_messages(conversation_id);
";

fn assert_pending_rebuild_recovers(temporary_only: bool) {
    let conn = Connection::open_in_memory().expect("open database");
    conn.execute_batch(
        "CREATE TABLE _migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT 'now'
         );
         INSERT INTO _migrations (version) VALUES (10);
         CREATE TABLE pending_transcriptions_v11 (
             note_id TEXT PRIMARY KEY,
             transcription_id TEXT,
             soniox_file_id TEXT,
             created_at TEXT NOT NULL
         );
         INSERT INTO pending_transcriptions_v11
             VALUES ('note', 'transcription', 'file', '2026-01-01');",
    )
    .expect("seed temporary table");
    if !temporary_only {
        conn.execute_batch(
            "CREATE TABLE pending_transcriptions (
                note_id TEXT PRIMARY KEY,
                transcription_id TEXT,
                soniox_file_id TEXT,
                created_at TEXT NOT NULL
             );
             INSERT INTO pending_transcriptions
                 VALUES ('note', 'transcription', 'file', '2026-01-01');",
        )
        .expect("seed original table");
    }

    apply_migrations(&conn, &[(11, PENDING_REBUILD)])
        .expect("repair migration");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM pending_transcriptions", [], |row| {
            row.get(0)
        })
        .expect("count pending transcriptions");
    assert_eq!(count, 1);
}

fn assert_conversation_rebuild_recovers(version: i64, temporary_only: bool) {
    let conn = Connection::open_in_memory().expect("open database");
    conn.execute_batch(&format!(
        "CREATE TABLE _migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT 'now'
         );
         INSERT INTO _migrations (version) VALUES ({});
         CREATE TABLE conversation_messages_new (
             id TEXT PRIMARY KEY,
             conversation_id TEXT NOT NULL,
             content TEXT NOT NULL
         );
         INSERT INTO conversation_messages_new VALUES ('message', 'conversation', 'saved');",
        version - 1
    ))
    .expect("seed temporary table");
    if !temporary_only {
        conn.execute_batch(
            "CREATE TABLE conversation_messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                content TEXT NOT NULL
             );
             INSERT INTO conversation_messages VALUES ('message', 'conversation', 'saved');",
        )
        .expect("seed original table");
    }

    apply_migrations(&conn, &[(version, CONVERSATION_REBUILD)])
        .expect("repair migration");
    let content: String = conn
        .query_row(
            "SELECT content FROM conversation_messages WHERE id = 'message'",
            [],
            |row| row.get(0),
        )
        .expect("read conversation message");
    assert_eq!(content, "saved");
    let index_exists: bool = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'index' AND name = 'idx_cm_conversation'
            )",
            [],
            |row| row.get(0),
        )
        .expect("read conversation index");
    assert!(index_exists);
}

#[test]
fn pending_rebuild_recovers_when_only_temporary_table_survives() {
    assert_pending_rebuild_recovers(true);
}

#[test]
fn pending_rebuild_discards_temporary_table_when_both_tables_survive() {
    assert_pending_rebuild_recovers(false);
}

#[test]
fn v18_rebuild_recovers_when_only_temporary_table_survives() {
    assert_conversation_rebuild_recovers(18, true);
}

#[test]
fn v18_rebuild_discards_temporary_table_when_both_tables_survive() {
    assert_conversation_rebuild_recovers(18, false);
}

#[test]
fn v22_rebuild_recovers_when_only_temporary_table_survives() {
    assert_conversation_rebuild_recovers(22, true);
}

#[test]
fn v22_rebuild_discards_temporary_table_when_both_tables_survive() {
    assert_conversation_rebuild_recovers(22, false);
}
