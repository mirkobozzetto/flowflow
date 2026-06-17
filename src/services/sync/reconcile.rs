use crate::db::chunk_repo::{content_hash, ChunkRecord};
use crate::db::Database;
use crate::services::constants::EMBEDDING_DIMS;
use crate::services::vectordb::{Chunk, VectorStore};
use std::collections::HashSet;

const BACKFILL_FLAG: &str = "chunks_backfilled_v10";

static RECONCILE_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn reconcile_running() -> bool {
    RECONCILE_RUNNING.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn is_backfilled(db: &Database) -> bool {
    db.get_setting(BACKFILL_FLAG).as_deref() == Some("true")
}

fn parse_att_owner(id: &str) -> Option<String> {
    let mut parts = id.splitn(3, ':');
    let kind = parts.next()?;
    let owner = parts.next()?;
    parts.next()?;
    if kind == "att" {
        Some(owner.to_string())
    } else {
        None
    }
}

fn row_to_record(
    owner_id: &str,
    owner_kind: &str,
    id: String,
    row: &Chunk,
) -> ChunkRecord {
    ChunkRecord {
        id,
        owner_id: owner_id.to_string(),
        owner_kind: owner_kind.to_string(),
        chunk_index: row.chunk_index,
        vector: row.vector.clone(),
        content_hash: content_hash(&row.chunk_text),
        chunk_text: row.chunk_text.clone(),
        title: row.title.clone(),
        tags: row.tags.clone(),
        created_at: row.created_at.clone(),
    }
}

fn owner_note_id(
    db: &Database,
    owner_id: &str,
    owner_kind: &str,
) -> Option<String> {
    if owner_kind == "note" {
        Some(owner_id.to_string())
    } else {
        db.get_attachment(owner_id)
            .ok()
            .flatten()
            .map(|a| a.note_id)
    }
}

fn record_to_chunk(note_id: &str, r: ChunkRecord) -> Option<Chunk> {
    if r.vector.len() != EMBEDDING_DIMS {
        eprintln!(
            "[reconcile] skip malformed chunk {} (dim {} != {EMBEDDING_DIMS})",
            r.id,
            r.vector.len()
        );
        return None;
    }
    Some(Chunk {
        id: r.id,
        note_id: note_id.to_string(),
        chunk_text: r.chunk_text,
        chunk_index: r.chunk_index,
        vector: r.vector,
        title: r.title,
        tags: r.tags,
        created_at: r.created_at,
    })
}

pub async fn backfill_legacy_chunks(
    db: &Database,
    store: &VectorStore,
) -> Result<(), String> {
    if db.get_setting(BACKFILL_FLAG).as_deref() == Some("true") {
        return Ok(());
    }
    let notes = db.list_notes()?;
    for note in &notes {
        let rows = store.fetch_note_rows(&note.id).await?;
        if rows.is_empty() {
            continue;
        }
        let mut note_records: Vec<ChunkRecord> = Vec::new();
        let mut att_records: std::collections::HashMap<
            String,
            Vec<ChunkRecord>,
        > = std::collections::HashMap::new();
        for row in &rows {
            if let Some(att_id) = parse_att_owner(&row.id) {
                att_records.entry(att_id.clone()).or_default().push(
                    row_to_record(&att_id, "attachment", row.id.clone(), row),
                );
            } else {
                let det_id = format!("note:{}:{}", note.id, row.chunk_index);
                note_records.push(row_to_record(&note.id, "note", det_id, row));
            }
        }
        if !note_records.is_empty()
            && db.count_chunks_for_owner(&note.id, "note")? == 0
        {
            db.replace_chunks(&note.id, "note", &note_records)?;
        }
        for (att_id, recs) in att_records {
            if db.get_attachment(&att_id)?.is_some()
                && db.count_chunks_for_owner(&att_id, "attachment")? == 0
            {
                db.replace_chunks(&att_id, "attachment", &recs)?;
            }
        }
    }
    db.set_setting(BACKFILL_FLAG, "true")?;
    eprintln!("[reconcile] backfill done ({} notes)", notes.len());
    Ok(())
}

pub async fn reconstruct_from_blob(
    db: &Database,
    store: &VectorStore,
) -> Result<usize, String> {
    let owners = db.distinct_chunk_owners()?;
    let mut chunks: Vec<Chunk> = Vec::new();
    for (owner_id, owner_kind) in owners {
        let note_id = match owner_note_id(db, &owner_id, &owner_kind) {
            Some(n) => n,
            None => continue,
        };
        for r in db.chunks_for_owner(&owner_id, &owner_kind)? {
            if let Some(c) = record_to_chunk(&note_id, r) {
                chunks.push(c);
            }
        }
    }
    let n = chunks.len();
    store.add_chunks(chunks).await?;
    let _ = store.ensure_fts_index().await;
    eprintln!("[reconcile] reconstructed {n} chunks from BLOB");
    Ok(n)
}

pub async fn reconcile_once(
    db: &Database,
    store: &VectorStore,
) -> Result<(), String> {
    // Self-heal (RFC 6: "orphelins -> suppression"): drop SQLite chunks whose
    // owning note/attachment no longer exists. Guarantees convergence: every
    // remaining SQLite chunk has a live owner that reconstruct can resolve, so
    // no id stays permanently in the "missing" set. Only removes chunks for
    // entities the user already deleted; never touches live data.
    for (owner_id, owner_kind) in db.distinct_chunk_owners()? {
        let alive = match owner_kind.as_str() {
            "note" => db.get_note(&owner_id)?.is_some(),
            "attachment" => db.get_attachment(&owner_id)?.is_some(),
            _ => true,
        };
        if !alive {
            db.delete_owner_chunks(&owner_id, &owner_kind)?;
        }
    }

    let sqlite_ids: HashSet<String> = db.all_chunk_ids()?.into_iter().collect();
    let lance_ids: HashSet<String> =
        store.all_ids().await?.into_iter().collect();

    let orphans: Vec<String> =
        lance_ids.difference(&sqlite_ids).cloned().collect();
    store.delete_ids(&orphans).await?;

    let missing: HashSet<String> =
        sqlite_ids.difference(&lance_ids).cloned().collect();
    if missing.is_empty() {
        eprintln!(
            "[reconcile] up to date ({} chunks, -{} orphans)",
            sqlite_ids.len(),
            orphans.len()
        );
        return Ok(());
    }

    let owners = db.distinct_chunk_owners()?;
    let mut to_add: Vec<Chunk> = Vec::new();
    for (owner_id, owner_kind) in owners {
        let note_id = match owner_note_id(db, &owner_id, &owner_kind) {
            Some(n) => n,
            None => continue,
        };
        for r in db.chunks_for_owner(&owner_id, &owner_kind)? {
            if missing.contains(&r.id) {
                if let Some(c) = record_to_chunk(&note_id, r) {
                    to_add.push(c);
                }
            }
        }
    }
    let added = to_add.len();
    store.add_chunks(to_add).await?;
    let _ = store.ensure_fts_index().await;
    eprintln!("[reconcile] +{added} missing, -{} orphans", orphans.len());
    Ok(())
}

pub fn run_boot_reconcile() {
    spawn_reconcile_pass();
}

// One full local pass (backfill flag-guarded + reconcile) on its own thread
// and runtime. Also fired after a sync session applied remote rows (T20):
// freshly landed chunk BLOBs get mirrored into LanceDB without waiting for
// the next boot.
pub fn spawn_reconcile_pass() {
    std::thread::spawn(move || {
        RECONCILE_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
        struct RunningGuard;
        impl Drop for RunningGuard {
            fn drop(&mut self) {
                RECONCILE_RUNNING
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let _guard = RunningGuard;
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[reconcile] runtime: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let db = match Database::open() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[reconcile] db open: {e}");
                    return;
                }
            };
            let store = match VectorStore::open().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[reconcile] store open: {e}");
                    return;
                }
            };
            if let Err(e) = backfill_legacy_chunks(&db, &store).await {
                eprintln!("[reconcile] backfill: {e}");
            }
            crate::services::embed::embed_missing_notes(&db, &store).await;
            if let Err(e) = reconcile_once(&db, &store).await {
                eprintln!("[reconcile] reconcile: {e}");
            }
        });
    });
}
