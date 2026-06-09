use rusqlite::Connection;
use uuid::Uuid;

// Multi-device sync change-tracking (RFC 0004, foundation T02/T03/T05).
//
// Design (post-review): the app opens MANY connections (Database::open at 9+
// sites + embed threads), so per-connection setup is unreliable. Instead the
// tracking lives in AFTER INSERT/UPDATE triggers whose DEFAULT behaviour is
// "local write": any connection that mutates a synced table records local
// authorship in sync_row_meta, with zero per-connection setup. device_id is
// read from a config row (settings) by the triggers at fire time. Only the
// dedicated sync-service connection flips a connection-local flag (via
// Database::set_applying), read by the triggers through the SQL function
// sync_is_applying(), which makes them no-op so it can write meta verbatim from a
// peer. (A trigger cannot read the temp schema, so the marker is a function, not
// a temp table.)
//
// Deletes are NOT tracked by AFTER DELETE triggers (the cascade-fires-trigger
// semantics are subtle); they are tombstoned applicatively via tombstone_entity
// in the repo delete paths, while CASCADE still does the physical cleanup.

pub const DEVICE_ID_KEY: &str = "sync_device_id";
const SEEDED_KEY: &str = "sync_meta_seeded";

// device_id subquery reused inside generated trigger bodies.
const DEV: &str = "(SELECT value FROM settings WHERE key='sync_device_id')";

struct Tracked {
    table: &'static str,
    kind: &'static str,
    new_id: &'static str,
    seed_id: &'static str,
    deleted_new: &'static str,
    deleted_seed: &'static str,
    update_trigger: bool,
}

const TRACKED: &[Tracked] = &[
    Tracked {
        table: "notes",
        kind: "note",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: true,
    },
    Tracked {
        table: "folders",
        kind: "folder",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: true,
    },
    Tracked {
        table: "notes_folders",
        kind: "notes_folders",
        new_id: "NEW.folder_id || ':' || NEW.note_id",
        seed_id: "t.folder_id || ':' || t.note_id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: false,
    },
    Tracked {
        table: "conversations",
        kind: "conversation",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: true,
    },
    Tracked {
        table: "conversation_messages",
        kind: "conversation_message",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: false,
    },
    Tracked {
        table: "attachments",
        kind: "attachment",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: true,
    },
    Tracked {
        table: "note_audios",
        kind: "note_audio",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "0",
        deleted_seed: "0",
        update_trigger: true,
    },
    Tracked {
        table: "note_reminders",
        kind: "note_reminder",
        new_id: "NEW.id",
        seed_id: "t.id",
        deleted_new: "CASE WHEN NEW.state='tombstone' THEN 1 ELSE 0 END",
        deleted_seed: "CASE WHEN t.state='tombstone' THEN 1 ELSE 0 END",
        update_trigger: true,
    },
];

// Body shared by AFTER INSERT and AFTER UPDATE triggers: allocate a local
// origin_seq atomically (single-writer WAL => increment-then-read is safe),
// then upsert sync_row_meta bumping THIS device's entry in the version_vector.
fn meta_body(kind: &str, id_expr: &str, deleted_expr: &str) -> String {
    format!(
        "  UPDATE sync_seq SET next_seq = next_seq + 1 WHERE device_id = {DEV};
  INSERT INTO sync_row_meta
    (entity_kind, entity_id, version_vector, origin_device, origin_seq, deleted, updated_hlc)
  VALUES (
    '{kind}', {id_expr},
    json_set('{{}}', '$.\"' || {DEV} || '\"', 1),
    {DEV},
    (SELECT next_seq FROM sync_seq WHERE device_id = {DEV}),
    {deleted_expr},
    strftime('%Y-%m-%dT%H:%M:%fZ','now')
  )
  ON CONFLICT(entity_kind, entity_id) DO UPDATE SET
    version_vector = json_set(
      COALESCE(sync_row_meta.version_vector, '{{}}'),
      '$.\"' || {DEV} || '\"',
      COALESCE(json_extract(sync_row_meta.version_vector, '$.\"' || {DEV} || '\"'), 0) + 1
    ),
    origin_device = {DEV},
    origin_seq = (SELECT next_seq FROM sync_seq WHERE device_id = {DEV}),
    deleted = {deleted_expr},
    updated_hlc = strftime('%Y-%m-%dT%H:%M:%fZ','now');"
    )
}

// Generate + install the INSERT/UPDATE tracking triggers. Called by the V10
// migrate hook. Idempotent (CREATE TRIGGER IF NOT EXISTS). Triggers no-op when
// sync_is_applying() is true (the sync connection applying a peer batch), or
// before device_id is set (defensive: never corrupt meta with a NULL author).
pub fn install_sync_triggers(conn: &Connection) -> Result<(), String> {
    // Connection-local guard: sync_is_applying() is a SQL function registered per
    // connection (db/mod.rs) returning 1 only on the sync-service connection while
    // it applies a remote batch. A trigger CANNOT read the temp schema
    // (sqlite_temp_master resolves to main.* and errors at fire time), so the
    // apply marker must be a function, not a temp table.
    let guard = "sync_is_applying() = 0";
    let mut sql = String::new();
    for t in TRACKED {
        sql.push_str(&format!(
            "CREATE TRIGGER IF NOT EXISTS trg_sync_{table}_ai
AFTER INSERT ON {table}
WHEN {guard} AND {DEV} IS NOT NULL
BEGIN
{body}
END;
",
            table = t.table,
            body = meta_body(t.kind, t.new_id, t.deleted_new),
        ));
        if t.update_trigger {
            sql.push_str(&format!(
                "CREATE TRIGGER IF NOT EXISTS trg_sync_{table}_au
AFTER UPDATE ON {table}
WHEN {guard} AND {DEV} IS NOT NULL
BEGIN
{body}
END;
",
                table = t.table,
                body = meta_body(t.kind, t.new_id, t.deleted_new),
            ));
        }
    }
    conn.execute_batch(&sql)
        .map_err(|e| format!("install sync triggers: {e}"))
}

// Ensure a stable, process-global device_id (config row read by triggers) and
// its sync_seq counter row. Idempotent across the many connections; concurrent
// opens converge on the first stored value (INSERT OR IGNORE then read back).
pub fn ensure_device_id(conn: &Connection) -> Result<String, String> {
    let candidate = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![DEVICE_ID_KEY, candidate],
    )
    .map_err(|e| format!("ensure device_id: {e}"))?;
    let device_id: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [DEVICE_ID_KEY],
            |r| r.get(0),
        )
        .map_err(|e| format!("read device_id: {e}"))?;
    conn.execute(
        "INSERT OR IGNORE INTO sync_seq (device_id, next_seq) VALUES (?1, 0)",
        [&device_id],
    )
    .map_err(|e| format!("ensure sync_seq row: {e}"))?;
    Ok(device_id)
}

fn seed_deleted(t: &Tracked) -> &'static str {
    t.deleted_seed
}

// One-time seed of sync_row_meta for pre-existing v1.0 rows so they are tracked
// (and therefore syncable) from day one. Guarded by a settings flag; safe to
// call on every open. Each seeded row gets a freshly reserved origin_seq.
pub fn seed_sync_meta(
    conn: &Connection,
    device_id: &str,
) -> Result<(), String> {
    let already: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1)",
            [SEEDED_KEY],
            |r| r.get(0),
        )
        .map_err(|e| format!("seed guard: {e}"))?;
    if already {
        return Ok(());
    }
    for t in TRACKED {
        let missing = format!(
            "NOT EXISTS (SELECT 1 FROM sync_row_meta m \
             WHERE m.entity_kind = '{kind}' AND m.entity_id = {id})",
            kind = t.kind,
            id = t.seed_id,
        );
        let n: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM {table} t WHERE {missing}",
                    table = t.table
                ),
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("seed count {}: {e}", t.table))?;
        if n == 0 {
            continue;
        }
        let base: i64 = conn
            .query_row(
                "SELECT next_seq FROM sync_seq WHERE device_id = ?1",
                [device_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("seed base: {e}"))?;
        conn.execute(
            "UPDATE sync_seq SET next_seq = next_seq + ?2 WHERE device_id = ?1",
            rusqlite::params![device_id, n],
        )
        .map_err(|e| format!("seed reserve {}: {e}", t.table))?;
        let insert = format!(
            "INSERT INTO sync_row_meta
                (entity_kind, entity_id, version_vector, origin_device, origin_seq, deleted, updated_hlc)
             SELECT '{kind}', {id},
                json_set('{{}}', '$.\"' || ?1 || '\"', 1),
                ?1,
                ?2 + row_number() OVER (ORDER BY t.ROWID),
                {deleted},
                strftime('%Y-%m-%dT%H:%M:%fZ','now')
             FROM {table} t
             WHERE {missing}",
            kind = t.kind,
            id = t.seed_id,
            deleted = seed_deleted(t),
            table = t.table,
        );
        conn.execute(&insert, rusqlite::params![device_id, base])
            .map_err(|e| format!("seed insert {}: {e}", t.table))?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, '1')",
        [SEEDED_KEY],
    )
    .map_err(|e| format!("seed flag: {e}"))?;
    Ok(())
}

// Applicative tombstone: mark an entity deleted in sync_row_meta (bump local
// version_vector + origin_seq). Called from repo delete paths for the entity
// and each cascaded child BEFORE the physical DELETE.
pub fn tombstone_entity(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
) -> Result<(), String> {
    bump_meta(conn, kind, entity_id, 1)
}

// Collect single-column string ids from a one-parameter query (e.g. children of
// a row about to be deleted). Empty/err -> empty vec (callers loop over it).
pub(crate) fn collect_ids(
    conn: &Connection,
    sql: &str,
    param: &str,
) -> Vec<String> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([param], |r| r.get::<_, String>(0))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

// Collect notes_folders links as encoded entity ids "folder_id:note_id" from a
// one-parameter query.
pub(crate) fn collect_links(
    conn: &Connection,
    sql: &str,
    param: &str,
) -> Vec<String> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([param], |r| {
        let folder: String = r.get(0)?;
        let note: String = r.get(1)?;
        Ok(format!("{folder}:{note}"))
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

// Mark an entity as locally updated (deleted=0), bumping its version. For changes
// that bypass a normal INSERT/UPDATE on the row, or to un-tombstone on the
// add-wins-over-delete rule (RFC 0004, T19). Not used by the foundation paths.
#[allow(dead_code)]
pub fn mark_entity_updated(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
) -> Result<(), String> {
    bump_meta(conn, kind, entity_id, 0)
}

fn bump_meta(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
    deleted: i64,
) -> Result<(), String> {
    let device_id: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [DEVICE_ID_KEY],
            |r| r.get(0),
        )
        .map_err(|e| format!("bump_meta device_id: {e}"))?;
    conn.execute(
        "UPDATE sync_seq SET next_seq = next_seq + 1 WHERE device_id = ?1",
        [&device_id],
    )
    .map_err(|e| format!("bump_meta seq: {e}"))?;
    let seq: i64 = conn
        .query_row(
            "SELECT next_seq FROM sync_seq WHERE device_id = ?1",
            [&device_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("bump_meta read seq: {e}"))?;
    conn.execute(
        "INSERT INTO sync_row_meta
            (entity_kind, entity_id, version_vector, origin_device, origin_seq, deleted, updated_hlc)
         VALUES (?1, ?2, json_set('{}', '$.\"' || ?3 || '\"', 1), ?3, ?4, ?5,
                 strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         ON CONFLICT(entity_kind, entity_id) DO UPDATE SET
            version_vector = json_set(
                COALESCE(sync_row_meta.version_vector, '{}'),
                '$.\"' || ?3 || '\"',
                COALESCE(json_extract(sync_row_meta.version_vector, '$.\"' || ?3 || '\"'), 0) + 1
            ),
            origin_device = ?3,
            origin_seq = ?4,
            deleted = ?5,
            updated_hlc = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        rusqlite::params![kind, entity_id, device_id, seq, deleted],
    )
    .map_err(|e| format!("bump_meta upsert: {e}"))?;
    Ok(())
}
