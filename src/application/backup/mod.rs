use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::VectorStore;

mod manifest;

pub use manifest::{
    current_platform, excluded_settings_description, Counts, Manifest,
    ManifestEntry, ARCHIVE_EXTENSION, ARCHIVE_FORMAT, ARCHIVE_VERSION,
    AUDIO_DIR_PREFIX, DB_ENTRY_PATH, EXCLUDED_TABLES, MANIFEST_PATH,
    MIN_SCHEMA_VERSION,
};

mod archive;
mod fs_util;
mod paths;
mod snapshot;
mod snapshot_db;

pub use archive::{archive_filename, build_archive};
pub use fs_util::{assert_no_sidecars, SIDECAR_SUFFIXES};
use fs_util::{crc32_of_file, fsync_dir, fsync_file, sidecar_paths};
use paths::restore_state_parent;
pub use paths::{
    activate_restore_lock, import_staging_dir, pending_restore_dir,
    restore_bak_dir, restore_lock_active, RestoreState, RESTORE_STATE_PATH,
};
pub use snapshot::{create_scrubbed_snapshot, ensure_chunks_backfilled};
pub use snapshot_db::{
    audio_paths_from_snapshot, open_read_only, snapshot_counts,
};

pub fn export_staging_root() -> PathBuf {
    std::env::temp_dir().join("flowflow_export")
}

pub async fn export_archive(
    db: &Database,
    store: &VectorStore,
) -> Result<PathBuf, String> {
    ensure_chunks_backfilled(db, store).await?;
    let root = export_staging_root();
    if root.exists() {
        std::fs::remove_dir_all(&root)
            .map_err(|e| format!("export staging sweep: {e}"))?;
    }
    let staging = root.join("staging");
    let snapshot = create_scrubbed_snapshot(
        &crate::infrastructure::persistence::db_path(),
        &staging,
    )?;
    let audio_dir = PathBuf::from(crate::infrastructure::audio::output_dir());
    let archive_path = root.join(archive_filename());
    let manifest = build_archive(&snapshot, &audio_dir, &archive_path)?;
    let _ = std::fs::remove_file(&snapshot);
    eprintln!(
        "[backup] export ready: {} ({} entries, {} audio missing)",
        archive_path.display(),
        manifest.entries.len(),
        manifest.audio_missing.len()
    );
    Ok(archive_path)
}

#[cfg(not(target_os = "ios"))]
pub fn save_archive_dialog(archive: &Path) -> Result<Option<PathBuf>, String> {
    let filename = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("flowflow-backup.ffbak.zip");
    let Some(dest) = rfd::FileDialog::new().set_file_name(filename).save_file()
    else {
        eprintln!("[backup] export save cancelled");
        return Ok(None);
    };
    std::fs::copy(archive, &dest)
        .map_err(|e| format!("export save copy: {e}"))?;
    reveal_in_file_manager(&dest);
    Ok(Some(dest))
}

#[cfg(not(target_os = "ios"))]
pub fn reveal_in_file_manager(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedImport {
    pub manifest: Manifest,
    pub staging_dir: PathBuf,
    pub staged_db: PathBuf,
    pub same_lineage: bool,
}

fn extract_archive(archive: &Path, staging: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("archive open: {e}"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("archive read (not a zip?): {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("archive entry {i}: {e}"))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(format!(
                "archive entry with unsafe path: {}",
                entry.name()
            ));
        };
        let dest = staging.join(relative);
        if entry.is_dir() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("extract mkdir: {e}"))?;
        }
        let mut out = std::fs::File::create(&dest)
            .map_err(|e| format!("extract create {}: {e}", dest.display()))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("extract write {}: {e}", dest.display()))?;
    }
    Ok(())
}

fn validate_staged_db(
    staged_db: &Path,
    manifest: &Manifest,
) -> Result<(), String> {
    assert_no_sidecars(staged_db)?;
    let conn = open_read_only(staged_db)?;
    let db_schema: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("staged schema check: {e}"))?;
    if db_schema != manifest.schema_version {
        return Err(format!(
            "archive inconsistent: db schema v{db_schema} != manifest v{}",
            manifest.schema_version
        ));
    }
    let db_device_id: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("staged device_id check: {e}"))?;
    if db_device_id != manifest.device_id {
        return Err(format!(
            "archive inconsistent: db device_id {db_device_id} != manifest {}",
            manifest.device_id
        ));
    }
    drop(conn);
    let counts = snapshot_counts(staged_db)?;
    if counts != manifest.counts {
        return Err(format!(
            "archive inconsistent: db counts {counts:?} != manifest {:?}",
            manifest.counts
        ));
    }
    let entry_paths: std::collections::HashSet<&str> =
        manifest.entries.iter().map(|e| e.path.as_str()).collect();
    for filename in audio_paths_from_snapshot(staged_db)? {
        let entry = format!("{AUDIO_DIR_PREFIX}{filename}");
        if !entry_paths.contains(entry.as_str())
            && !manifest.audio_missing.contains(&filename)
        {
            return Err(format!(
                "archive inconsistent: audio {filename} neither embedded nor declared missing"
            ));
        }
    }
    Ok(())
}

pub fn validate_archive(
    archive: &Path,
    live_device_id: Option<&str>,
) -> Result<ValidatedImport, String> {
    validate_archive_at(
        archive,
        &import_staging_dir(),
        &pending_restore_dir(),
        live_device_id,
    )
}

pub fn validate_archive_at(
    archive: &Path,
    staging_dir: &Path,
    pending_dir: &Path,
    live_device_id: Option<&str>,
) -> Result<ValidatedImport, String> {
    if pending_dir.exists() {
        return Err(
            "a restore is already pending: restart the app first".to_string()
        );
    }
    let staging = staging_dir.to_path_buf();
    if staging.exists() {
        std::fs::remove_dir_all(&staging)
            .map_err(|e| format!("import staging reset: {e}"))?;
    }
    std::fs::create_dir_all(&staging)
        .map_err(|e| format!("import staging create: {e}"))?;

    extract_archive(archive, &staging)?;

    let manifest_file = staging.join(MANIFEST_PATH);
    let raw = std::fs::read_to_string(&manifest_file)
        .map_err(|e| format!("manifest missing in archive: {e}"))?;
    let manifest = Manifest::from_json(&raw)?;

    if manifest.format != ARCHIVE_FORMAT {
        return Err(format!("not a FlowFlow backup: {}", manifest.format));
    }
    if manifest.archive_version > ARCHIVE_VERSION {
        return Err(format!(
            "archive version {} is newer than this app supports ({ARCHIVE_VERSION}): update FlowFlow",
            manifest.archive_version
        ));
    }
    let app_schema =
        crate::infrastructure::persistence::current_schema_version();
    if manifest.schema_version > app_schema {
        return Err(format!(
            "archive schema v{} is newer than this app (v{app_schema}): update FlowFlow",
            manifest.schema_version
        ));
    }
    if manifest.schema_version < MIN_SCHEMA_VERSION {
        return Err(format!(
            "archive schema v{} predates v{MIN_SCHEMA_VERSION}: unsupported (no embedded vectors)",
            manifest.schema_version
        ));
    }

    let mut expected: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();
    expected.insert(PathBuf::from(MANIFEST_PATH));
    let mut has_db_entry = false;
    for entry in &manifest.entries {
        if entry.path == DB_ENTRY_PATH {
            has_db_entry = true;
        } else if !entry.path.starts_with(AUDIO_DIR_PREFIX) {
            return Err(format!("unexpected manifest entry: {}", entry.path));
        }
        let extracted = staging.join(&entry.path);
        let crc = crc32_of_file(&extracted)
            .map_err(|_| format!("entry missing in archive: {}", entry.path))?;
        if crc != entry.crc32 {
            return Err(format!(
                "crc mismatch for {}: archive corrupted",
                entry.path
            ));
        }
        expected.insert(PathBuf::from(&entry.path));
    }
    if !has_db_entry {
        return Err("archive has no database entry".to_string());
    }
    let mut stack = vec![staging.clone()];
    while let Some(dir) = stack.pop() {
        for child in std::fs::read_dir(&dir)
            .map_err(|e| format!("staging scan: {e}"))?
            .flatten()
        {
            let path = child.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(&staging)
                    .map_err(|e| format!("staging rel: {e}"))?;
                if !expected.contains(rel) {
                    return Err(format!(
                        "unexpected file in archive: {}",
                        rel.display()
                    ));
                }
            }
        }
    }

    let staged_db = staging.join(DB_ENTRY_PATH);
    validate_staged_db(&staged_db, &manifest)?;

    let same_lineage = live_device_id
        .map(|id| id == manifest.device_id)
        .unwrap_or(false);
    eprintln!(
        "[backup] archive validated: schema v{}, {} entries, {} audio missing, lineage {}",
        manifest.schema_version,
        manifest.entries.len(),
        manifest.audio_missing.len(),
        if same_lineage { "same" } else { "other" }
    );
    Ok(ValidatedImport {
        manifest,
        staging_dir: staging,
        staged_db,
        same_lineage,
    })
}

pub fn stage_import(validated: &ValidatedImport) -> Result<(), String> {
    stage_import_at(validated, &pending_restore_dir())
}

pub fn stage_import_at(
    validated: &ValidatedImport,
    pending_dir: &Path,
) -> Result<(), String> {
    let pending = pending_dir.to_path_buf();
    if pending.exists() {
        return Err(
            "a restore is already pending: restart the app first".to_string()
        );
    }

    {
        let conn = Connection::open(&validated.staged_db)
            .map_err(|e| format!("stage markers open: {e}"))?;
        conn.pragma_update(None, "journal_mode", "MEMORY")
            .map_err(|e| format!("stage markers journal: {e}"))?;
        let device_id: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'sync_device_id'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("stage markers device_id: {e}"))?;
        let floor: i64 = conn
            .query_row(
                "SELECT COALESCE(
                    (SELECT next_seq FROM sync_seq WHERE device_id = ?1), 0)",
                [&device_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("stage markers floor: {e}"))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('sync_restored_pending', 'true')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .map_err(|e| format!("stage markers pending: {e}"))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('sync_restored_floor', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [floor.to_string()],
        )
        .map_err(|e| format!("stage markers floor write: {e}"))?;
        eprintln!("[backup] restore markers staged (floor {floor})");

        drop(conn);
        let state = RestoreState {
            staged_db_crc32: crc32_of_file(&validated.staged_db)?,
            floor,
        };
        let state_json = serde_json::to_string(&state)
            .map_err(|e| format!("restore state serialize: {e}"))?;
        let state_file = validated.staging_dir.join(RESTORE_STATE_PATH);
        std::fs::write(&state_file, state_json)
            .map_err(|e| format!("restore state write: {e}"))?;
        fsync_file(&state_file)?;
    }
    assert_no_sidecars(&validated.staged_db)?;

    fsync_file(&validated.staged_db)?;
    if let Some(db_parent) = validated.staged_db.parent() {
        fsync_dir(db_parent)?;
    }
    fsync_dir(&validated.staging_dir)?;

    std::fs::rename(&validated.staging_dir, &pending)
        .map_err(|e| format!("stage commit rename: {e}"))?;
    if let Some(parent) = pending.parent() {
        fsync_dir(parent)?;
    }
    activate_restore_lock();
    eprintln!("[backup] import staged at {}", pending.display());
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum RestoreOutcome {
    None,
    Committed,
    RolledBack { reason: String },
}

pub struct RestorePaths {
    pub pending: PathBuf,
    pub bak: PathBuf,
    pub data_db: PathBuf,
    pub audio_dir: PathBuf,
    pub vectordb_dir: PathBuf,
    pub error_file: PathBuf,
}

pub fn default_restore_paths() -> RestorePaths {
    RestorePaths {
        pending: pending_restore_dir(),
        bak: restore_bak_dir(),
        data_db: crate::infrastructure::persistence::raw_db_path(),
        audio_dir: PathBuf::from(crate::infrastructure::audio::output_dir()),
        vectordb_dir: PathBuf::from(
            crate::infrastructure::vectordb::vectordb_path(),
        ),
        error_file: restore_state_parent().join("restore_error.txt"),
    }
}

pub fn apply_pending_restore_or_abort() {
    match apply_pending_restore_at(&default_restore_paths()) {
        Ok(RestoreOutcome::None) => {}
        Ok(RestoreOutcome::Committed) => {
            eprintln!("[backup] restore committed");
        }
        Ok(RestoreOutcome::RolledBack { reason }) => {
            eprintln!("[backup] restore rolled back: {reason}");
        }
        Err(e) => {
            eprintln!(
                "[backup] FATAL: restore failed and rollback failed: {e}. \
                 Aborting boot: continuing would risk reinstalling a stale \
                 store over the only intact copy."
            );
            std::process::exit(78);
        }
    }
}

pub fn apply_pending_restore_at(
    p: &RestorePaths,
) -> Result<RestoreOutcome, String> {
    if !p.pending.exists() {
        return Ok(RestoreOutcome::None);
    }
    let pending_db = p.pending.join(DB_ENTRY_PATH);
    if !pending_db.exists() {
        if p.data_db.exists() {
            std::fs::remove_dir_all(&p.pending)
                .map_err(|e| format!("commit leftover cleanup: {e}"))?;
            eprintln!("[backup] restore commit leftover cleaned");
            return Ok(RestoreOutcome::Committed);
        }
        let reason = "pending dir without staged db and no data db".to_string();
        rollback(p, &reason)?;
        return Ok(RestoreOutcome::RolledBack { reason });
    }
    match run_swap(p, &pending_db) {
        Ok(()) => Ok(RestoreOutcome::Committed),
        Err(reason) => {
            eprintln!("[backup] swap failed: {reason}; rolling back");
            rollback(p, &reason)?;
            Ok(RestoreOutcome::RolledBack { reason })
        }
    }
}

fn run_swap(p: &RestorePaths, pending_db: &Path) -> Result<(), String> {
    let manifest_raw = std::fs::read_to_string(p.pending.join(MANIFEST_PATH))
        .map_err(|e| format!("phase2 manifest read: {e}"))?;
    let manifest = Manifest::from_json(&manifest_raw)?;
    let state_raw = std::fs::read_to_string(p.pending.join(RESTORE_STATE_PATH))
        .map_err(|e| format!("phase2 state read: {e}"))?;
    let state: RestoreState = serde_json::from_str(&state_raw)
        .map_err(|e| format!("phase2 state parse: {e}"))?;
    assert_no_sidecars(pending_db)?;
    let staged_crc = crc32_of_file(pending_db)?;
    if staged_crc != state.staged_db_crc32 {
        return Err(format!(
            "staged db crc mismatch (got {staged_crc}, want {})",
            state.staged_db_crc32
        ));
    }

    std::fs::create_dir_all(&p.audio_dir)
        .map_err(|e| format!("phase2 audio dir: {e}"))?;
    let bak_audio = p.bak.join("audio");
    let mut restored_wavs = 0usize;
    for entry in &manifest.entries {
        let Some(filename) = entry.path.strip_prefix(AUDIO_DIR_PREFIX) else {
            continue;
        };
        let src = p.pending.join(&entry.path);
        if !src.is_file() {
            return Err(format!("phase2 wav missing in pending: {filename}"));
        }
        let target = p.audio_dir.join(filename);
        if target.exists() {
            let target_crc = crc32_of_file(&target)?;
            if target_crc == entry.crc32 {
                continue;
            }
            let set_aside = bak_audio.join(filename);
            if set_aside.exists() {
                std::fs::remove_file(&target).map_err(|e| {
                    format!("phase2 stale target {filename}: {e}")
                })?;
            } else {
                std::fs::create_dir_all(&bak_audio)
                    .map_err(|e| format!("phase2 bak audio dir: {e}"))?;
                std::fs::rename(&target, &set_aside)
                    .map_err(|e| format!("phase2 set aside {filename}: {e}"))?;
            }
        }
        std::fs::copy(&src, &target)
            .map_err(|e| format!("phase2 wav copy {filename}: {e}"))?;
        let copied_crc = crc32_of_file(&target)?;
        if copied_crc != entry.crc32 {
            return Err(format!("phase2 wav copy corrupted: {filename}"));
        }
        fsync_file(&target)?;
        restored_wavs += 1;
        eprintln!("[backup] phase2 wav restored: {filename}");
    }
    fsync_dir(&p.audio_dir)?;

    if p.data_db.exists() {
        let conn = Connection::open(&p.data_db)
            .map_err(|e| format!("phase2 old db open: {e}"))?;
        let _busy: i64 = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
            .map_err(|e| format!("phase2 checkpoint: {e}"))?;
        drop(conn);
        for sidecar in sidecar_paths(&p.data_db) {
            if sidecar.exists() {
                std::fs::remove_file(&sidecar)
                    .map_err(|e| format!("phase2 sidecar sweep: {e}"))?;
            }
        }
        std::fs::create_dir_all(&p.bak)
            .map_err(|e| format!("phase2 bak dir: {e}"))?;
        let bak_db = p.bak.join("flowflow.db");
        std::fs::rename(&p.data_db, &bak_db)
            .map_err(|e| format!("phase2 set aside old db: {e}"))?;
        eprintln!("[backup] phase2 old db set aside");
    }

    if p.vectordb_dir.exists() {
        std::fs::remove_dir_all(&p.vectordb_dir)
            .map_err(|e| format!("phase2 vectordb purge: {e}"))?;
        eprintln!("[backup] phase2 vectordb purged (derived cache)");
    }

    for sidecar in sidecar_paths(&p.data_db) {
        if sidecar.exists() {
            std::fs::remove_file(&sidecar)
                .map_err(|e| format!("phase2 retry sidecar sweep: {e}"))?;
        }
    }
    let data_dir = p
        .data_db
        .parent()
        .ok_or_else(|| "phase2 data dir".to_string())?;
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("phase2 data dir create: {e}"))?;
    fsync_dir(data_dir)?;
    std::fs::rename(pending_db, &p.data_db)
        .map_err(|e| format!("phase2 commit rename: {e}"))?;
    fsync_dir(data_dir)?;
    eprintln!(
        "[backup] phase2 committed ({restored_wavs} wav restored this run)"
    );

    std::fs::remove_dir_all(&p.pending)
        .map_err(|e| format!("phase2 pending cleanup: {e}"))?;
    Ok(())
}

fn rollback(p: &RestorePaths, reason: &str) -> Result<(), String> {
    eprintln!("[backup] rollback: {reason}");
    let bak_db = p.bak.join("flowflow.db");
    if bak_db.exists() {
        if let Some(parent) = p.data_db.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("rollback data dir: {e}"))?;
        }
        std::fs::rename(&bak_db, &p.data_db)
            .map_err(|e| format!("rollback db restore: {e}"))?;
    }
    let bak_audio = p.bak.join("audio");
    if bak_audio.exists() {
        for entry in std::fs::read_dir(&bak_audio)
            .map_err(|e| format!("rollback audio scan: {e}"))?
            .flatten()
        {
            let target = p.audio_dir.join(entry.file_name());
            std::fs::rename(entry.path(), &target)
                .map_err(|e| format!("rollback wav restore: {e}"))?;
        }
    }
    if p.bak.exists() {
        std::fs::remove_dir_all(&p.bak)
            .map_err(|e| format!("rollback bak cleanup: {e}"))?;
    }
    if p.pending.exists() {
        std::fs::remove_dir_all(&p.pending)
            .map_err(|e| format!("rollback pending cleanup: {e}"))?;
    }
    if !p.data_db.exists() {
        return Err(format!(
            "rollback finished but no db at {}",
            p.data_db.display()
        ));
    }
    let message = format!(
        "restore failed and was rolled back: {reason} ({})",
        crate::infrastructure::persistence::now_iso()
    );
    let _ = std::fs::write(&p.error_file, &message);
    Ok(())
}

pub fn take_restore_error() -> Option<String> {
    let file = restore_state_parent().join("restore_error.txt");
    let message = std::fs::read_to_string(&file).ok()?;
    let _ = std::fs::remove_file(&file);
    Some(message)
}

pub fn restore_recovery_window_active() -> bool {
    restore_bak_dir().exists()
}

pub fn finalize_restore_bak() {
    finalize_restore_bak_at(&restore_bak_dir());
}

pub fn finalize_restore_bak_at(bak: &Path) {
    if !bak.exists() {
        return;
    }
    let stamp = bak.join(".boot_survived");
    if stamp.exists() {
        match std::fs::remove_dir_all(bak) {
            Ok(()) => eprintln!(
                "[backup] restore_bak purged (second successful boot)"
            ),
            Err(e) => eprintln!("[backup] restore_bak purge failed: {e}"),
        }
    } else {
        match std::fs::write(&stamp, b"1") {
            Ok(()) => eprintln!(
                "[backup] restore_bak retained until next successful boot"
            ),
            Err(e) => eprintln!("[backup] restore_bak stamp failed: {e}"),
        }
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn manifest_round_trips_through_json() {
        let mut manifest = Manifest::new(
            "device-abc".into(),
            Counts {
                notes: 3,
                folders: 1,
                threads: 1,
                attachments: 2,
                conversations: 4,
                audio_files: 5,
                chunks: 42,
                reminders: 6,
            },
        );
        manifest.audio_missing.push("recording_1.wav".into());
        manifest.entries.push(ManifestEntry {
            path: DB_ENTRY_PATH.into(),
            crc32: 0xDEADBEEF,
        });

        let json = manifest.to_json().unwrap();
        let parsed = Manifest::from_json(&json).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn schema_version_tracks_migrations_max() {
        let manifest = Manifest::new("d".into(), Counts::default());
        assert_eq!(
            manifest.schema_version,
            crate::infrastructure::persistence::current_schema_version()
        );
        assert!(manifest.schema_version >= MIN_SCHEMA_VERSION);
    }

    #[test]
    fn manifest_carries_format_and_device_id() {
        let manifest = Manifest::new("lineage-1".into(), Counts::default());
        assert_eq!(manifest.format, ARCHIVE_FORMAT);
        assert_eq!(manifest.archive_version, ARCHIVE_VERSION);
        assert_eq!(manifest.device_id, "lineage-1");
        assert!(!manifest.app_version.is_empty());
    }

    #[test]
    fn excluded_settings_cover_all_scrub_lists() {
        let desc = excluded_settings_description();
        for key in SENSITIVE_SETTINGS
            .iter()
            .chain(DEVICE_LOCAL_SETTINGS.iter())
        {
            assert!(desc.contains(&key.to_string()), "missing {key}");
        }
        for prefix in SENSITIVE_SETTING_PREFIXES
            .iter()
            .chain(DEVICE_LOCAL_SETTING_PREFIXES.iter())
        {
            assert!(desc.contains(&format!("{prefix}*")), "missing {prefix}*");
        }
    }
}

#[cfg(all(test, not(target_os = "ios")))]
mod snapshot_tests {
    use super::*;
    use crate::domain::note::NewTextNote;

    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn seed_source(dir: &Path) -> (PathBuf, Vec<String>) {
        let db_file = dir.join("flowflow.db");
        let db = Database::open_at(db_file.clone()).expect("source db");
        db.create_text_note(&NewTextNote {
            title: Some("kept".into()),
            content: "note content survives the scrub".into(),
            tags: vec![],
        })
        .expect("seed note");
        let secrets = vec![
            ("openai_api_key", "sk-fake-test-openai-key-for-scrub-check"),
            ("anthropic_api_key", "sk-ant-fake-test-anthropic-key"),
            ("soniox_api_key", "fake-test-soniox-key-value"),
            ("sync_static_privkey", "fake-test-noise-privkey-sentinel"),
            ("sync_static_pubkey", "fake-test-noise-pubkey-sentinel"),
            ("sync_psk_peer-1", "fake-test-psk-sentinel-peer-one"),
        ];
        for (key, value) in &secrets {
            db.set_setting(key, value).expect("seed secret");
        }
        db.set_setting("sync_peer_addr_peer-1", "10.0.0.5:48653")
            .unwrap();
        db.set_setting("sync_peer_acked_by_peer-1", "41").unwrap();
        db.set_setting("ai_consent", "accepted").unwrap();
        db.set_setting("sync_restored_pending", "true").unwrap();
        db.set_setting("sync_restored_floor", "7").unwrap();
        db.set_setting("sync_restored_done_peer-1", "true").unwrap();
        db.set_setting("language", "fr").unwrap();
        db.set_setting("llm_provider", "openai").unwrap();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO sync_peers (device_id, static_pubkey, last_acked_seq, gc_horizon)
                 VALUES ('peer-1', 'peer-pubkey', 5, 2)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO pending_transcriptions (note_id, transcription_id)
                 VALUES ('n1', 't1')",
                [],
            )
            .unwrap();
        }
        let captured = secrets.iter().map(|(_, v)| v.to_string()).collect();
        (db_file, captured)
    }

    #[test]
    fn snapshot_scrub_removes_secrets_and_peer_state() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let staging = staging_dir.path().join("export_staging");
        let (db_file, captured) = seed_source(source_dir.path());

        let snapshot =
            create_scrubbed_snapshot(&db_file, &staging).expect("snapshot");

        let bytes = std::fs::read(&snapshot).unwrap();
        for secret in &captured {
            assert!(
                !contains_bytes(&bytes, secret.as_bytes()),
                "captured secret leaked into snapshot: {secret}"
            );
        }

        let conn = open_read_only(&snapshot).unwrap();
        let excluded_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key IN
                 ('openai_api_key','anthropic_api_key','soniox_api_key',
                  'sync_static_privkey','sync_static_pubkey','ai_consent',
                  'sync_restored_pending','sync_restored_floor')
                 OR substr(key, 1, 9) = 'sync_psk_'
                 OR substr(key, 1, 15) = 'sync_peer_addr_'
                 OR substr(key, 1, 19) = 'sync_peer_acked_by_'
                 OR substr(key, 1, 19) = 'sync_restored_done_'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(excluded_count, 0, "excluded settings must be scrubbed");
        let peers: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_peers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(peers, 0);
        let pending: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transcriptions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(pending, 0);

        let language: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'language'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(language, "fr");
        let device_id: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'sync_device_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(device_id, 1, "data identity must survive the scrub");
        let notes: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 1, "user data must survive the scrub");
    }

    #[test]
    fn snapshot_leaves_no_sidecar_and_source_intact() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let staging = staging_dir.path().join("export_staging");
        let (db_file, _) = seed_source(source_dir.path());

        let snapshot =
            create_scrubbed_snapshot(&db_file, &staging).expect("snapshot");

        assert_no_sidecars(&snapshot).expect("no sidecars next to snapshot");
        let entries: Vec<String> = std::fs::read_dir(&staging)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["flowflow.db".to_string()]);

        let source = Database::open_at(db_file).unwrap();
        assert_eq!(source.list_notes().unwrap().len(), 1);
        assert_eq!(
            source.get_setting("openai_api_key").as_deref(),
            Some("sk-fake-test-openai-key-for-scrub-check"),
            "source DB must keep its secrets"
        );
        let peers: i64 = source
            .conn()
            .query_row("SELECT COUNT(*) FROM sync_peers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(peers, 1, "source peer state must be untouched");
    }

    #[test]
    fn staging_dir_is_recreated_from_scratch() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let staging = staging_dir.path().join("export_staging");
        let (db_file, _) = seed_source(source_dir.path());

        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("flowflow.db-journal"), b"stale secret")
            .unwrap();

        let snapshot =
            create_scrubbed_snapshot(&db_file, &staging).expect("snapshot");

        assert_no_sidecars(&snapshot).unwrap();
        assert!(!staging.join("flowflow.db-journal").exists());
    }

    #[test]
    fn snapshot_counts_reflect_scrubbed_copy() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let staging = staging_dir.path().join("export_staging");
        let (db_file, _) = seed_source(source_dir.path());

        let snapshot =
            create_scrubbed_snapshot(&db_file, &staging).expect("snapshot");
        let counts = snapshot_counts(&snapshot).unwrap();
        assert_eq!(counts.notes, 1);
        assert_eq!(counts.folders, 0);
        assert_eq!(counts.audio_files, 0);
        assert_eq!(counts.chunks, 0);
    }
}

#[cfg(all(test, not(target_os = "ios")))]
mod archive_tests {
    use super::*;
    use crate::domain::note::NewTextNote;

    fn seed_with_audio(dir: &Path, audio_dir: &Path) -> PathBuf {
        let db_file = dir.join("flowflow.db");
        let db = Database::open_at(db_file.clone()).expect("db");
        let note = db
            .create_text_note(&NewTextNote {
                title: Some("audio note".into()),
                content: "has recordings".into(),
                tags: vec![],
            })
            .unwrap();
        std::fs::create_dir_all(audio_dir).unwrap();
        std::fs::write(audio_dir.join("recording_100.wav"), b"RIFFfakewav100")
            .unwrap();
        db.add_audio(&note.id, "recording_100.wav", 1.5).unwrap();
        db.add_audio(&note.id, "recording_200.wav", 2.5).unwrap();
        db_file
    }

    fn archive_entry_names(archive_path: &Path) -> Vec<String> {
        let file = std::fs::File::open(archive_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect()
    }

    #[test]
    fn archive_contains_explicit_entries_only() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let audio_dir = source_dir.path().join("audio");
        let staging = staging_dir.path().join("export_staging");
        let db_file = seed_with_audio(source_dir.path(), &audio_dir);

        let snapshot = create_scrubbed_snapshot(&db_file, &staging).unwrap();
        std::fs::write(staging.join("stray-secret.txt"), b"never zipped")
            .unwrap();
        let archive_path = staging_dir.path().join("backup.ffbak.zip");
        let manifest =
            build_archive(&snapshot, &audio_dir, &archive_path).unwrap();

        let mut names = archive_entry_names(&archive_path);
        names.sort();
        assert_eq!(
            names,
            vec![
                "audio/recording_100.wav".to_string(),
                "db/flowflow.db".to_string(),
                "manifest.json".to_string(),
            ]
        );
        assert_eq!(
            manifest.audio_missing,
            vec!["recording_200.wav".to_string()]
        );
        assert_eq!(manifest.counts.audio_files, 2);
        assert_eq!(manifest.counts.notes, 1);
    }

    #[test]
    fn archive_never_contains_whisper_model_files() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let audio_dir = source_dir.path().join("audio");
        let staging = staging_dir.path().join("export_staging");
        let db_file = seed_with_audio(source_dir.path(), &audio_dir);

        let db = Database::open_at(db_file.clone()).unwrap();
        db.set_setting("stt_provider", "whisper_local").unwrap();
        db.set_setting("whisper_model", "tiny").unwrap();
        drop(db);

        let models_dir = source_dir.path().join("models/whisper");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::write(models_dir.join("ggml-tiny.bin"), b"fake model")
            .unwrap();
        std::fs::write(
            audio_dir.join("ggml-small-q5_1.bin"),
            b"model hiding in audio dir",
        )
        .unwrap();

        let snapshot = create_scrubbed_snapshot(&db_file, &staging).unwrap();
        let archive_path = staging_dir.path().join("backup.ffbak.zip");
        build_archive(&snapshot, &audio_dir, &archive_path).unwrap();

        let names = archive_entry_names(&archive_path);
        assert!(
            names
                .iter()
                .all(|n| !n.contains("model") && !n.ends_with(".bin")),
            "no model file may enter the archive: {names:?}"
        );

        let extract_dir = tempfile::tempdir().unwrap();
        let out_path = extract_dir.path().join("restored.db");
        {
            let file = std::fs::File::open(&archive_path).unwrap();
            let mut zip = zip::ZipArchive::new(file).unwrap();
            let mut entry = zip.by_name("db/flowflow.db").unwrap();
            let mut out = std::fs::File::create(&out_path).unwrap();
            std::io::copy(&mut entry, &mut out).unwrap();
        }
        let restored = Database::open_at(out_path).unwrap();
        assert_eq!(
            restored.get_setting("stt_provider").as_deref(),
            Some("whisper_local"),
            "provider choice must survive the backup"
        );
        assert_eq!(
            restored.get_setting("whisper_model").as_deref(),
            Some("tiny"),
            "active model id must survive the backup"
        );
    }

    #[test]
    fn manifest_crc_matches_zip_entry_crc() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let audio_dir = source_dir.path().join("audio");
        let staging = staging_dir.path().join("export_staging");
        let db_file = seed_with_audio(source_dir.path(), &audio_dir);

        let snapshot = create_scrubbed_snapshot(&db_file, &staging).unwrap();
        let archive_path = staging_dir.path().join("backup.ffbak.zip");
        let manifest =
            build_archive(&snapshot, &audio_dir, &archive_path).unwrap();

        let file = std::fs::File::open(&archive_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        for entry in &manifest.entries {
            let zipped = zip.by_name(&entry.path).unwrap();
            assert_eq!(
                zipped.crc32(),
                entry.crc32,
                "crc mismatch for {}",
                entry.path
            );
        }
        let manifest_in_zip = {
            use std::io::Read;
            let mut raw = String::new();
            zip.by_name(MANIFEST_PATH)
                .unwrap()
                .read_to_string(&mut raw)
                .unwrap();
            Manifest::from_json(&raw).unwrap()
        };
        assert_eq!(manifest_in_zip, manifest);
    }

    #[test]
    fn manifest_device_id_comes_from_snapshot() {
        let source_dir = tempfile::tempdir().unwrap();
        let staging_dir = tempfile::tempdir().unwrap();
        let audio_dir = source_dir.path().join("audio");
        let staging = staging_dir.path().join("export_staging");
        let db_file = seed_with_audio(source_dir.path(), &audio_dir);

        let live_device_id = Database::open_at(db_file.clone())
            .unwrap()
            .get_setting("sync_device_id")
            .unwrap();
        let snapshot = create_scrubbed_snapshot(&db_file, &staging).unwrap();
        let archive_path = staging_dir.path().join("backup.ffbak.zip");
        let manifest =
            build_archive(&snapshot, &audio_dir, &archive_path).unwrap();
        assert_eq!(manifest.device_id, live_device_id);
    }
}

#[cfg(all(test, not(target_os = "ios")))]
mod import_tests {
    use super::*;
    use crate::domain::note::NewTextNote;

    struct Fixture {
        _source_dir: tempfile::TempDir,
        _work_dir: tempfile::TempDir,
        archive: PathBuf,
        staging: PathBuf,
        pending: PathBuf,
        source_device_id: String,
    }

    fn build_fixture() -> Fixture {
        let source_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_file = source_dir.path().join("flowflow.db");
        let audio_dir = source_dir.path().join("audio");
        let db = Database::open_at(db_file.clone()).unwrap();
        let note = db
            .create_text_note(&NewTextNote {
                title: Some("roundtrip".into()),
                content: "import me".into(),
                tags: vec![],
            })
            .unwrap();
        std::fs::create_dir_all(&audio_dir).unwrap();
        std::fs::write(audio_dir.join("recording_300.wav"), b"RIFFwav300")
            .unwrap();
        db.add_audio(&note.id, "recording_300.wav", 1.0).unwrap();
        db.set_setting("openai_api_key", "sk-secret-not-in-archive")
            .unwrap();
        let source_device_id = db.get_setting("sync_device_id").unwrap();
        drop(db);

        let export_staging = work_dir.path().join("export_staging");
        let snapshot =
            create_scrubbed_snapshot(&db_file, &export_staging).unwrap();
        let archive = work_dir.path().join("backup.ffbak.zip");
        build_archive(&snapshot, &audio_dir, &archive).unwrap();

        Fixture {
            archive,
            staging: work_dir.path().join("import_staging"),
            pending: work_dir.path().join("pending_restore"),
            _source_dir: source_dir,
            _work_dir: work_dir,
            source_device_id,
        }
    }

    #[test]
    fn valid_archive_passes_validation_without_mutating_staged_bytes() {
        let fx = build_fixture();
        let validated = validate_archive_at(
            &fx.archive,
            &fx.staging,
            &fx.pending,
            Some(&fx.source_device_id),
        )
        .expect("valid archive must validate");
        assert!(validated.same_lineage);
        assert_eq!(validated.manifest.counts.notes, 1);

        let before = std::fs::read(&validated.staged_db).unwrap();
        let again =
            validate_staged_db(&validated.staged_db, &validated.manifest);
        assert!(again.is_ok());
        let after = std::fs::read(&validated.staged_db).unwrap();
        assert_eq!(before, after, "validation must be 100% read-only");
        assert_no_sidecars(&validated.staged_db).unwrap();
    }

    #[test]
    fn other_lineage_is_detected() {
        let fx = build_fixture();
        let validated = validate_archive_at(
            &fx.archive,
            &fx.staging,
            &fx.pending,
            Some("another-device-entirely"),
        )
        .unwrap();
        assert!(!validated.same_lineage);
    }

    #[test]
    fn corrupted_entry_is_refused() {
        let fx = build_fixture();
        use std::io::Write;
        let raw = std::fs::File::open(&fx.archive).unwrap();
        let mut src = zip::ZipArchive::new(raw).unwrap();
        let tampered_path = fx.archive.with_file_name("tampered.ffbak.zip");
        let out = std::fs::File::create(&tampered_path).unwrap();
        let mut writer = zip::ZipWriter::new(out);
        let options = zip::write::SimpleFileOptions::default();
        for i in 0..src.len() {
            let entry = src.by_index_raw(i).unwrap();
            if entry.name() == "audio/recording_300.wav" {
                continue;
            }
            writer.raw_copy_file(entry).unwrap();
        }
        writer
            .start_file("audio/recording_300.wav", options)
            .unwrap();
        writer.write_all(b"RIFFtampered-bytes").unwrap();
        writer.finish().unwrap();

        let err =
            validate_archive_at(&tampered_path, &fx.staging, &fx.pending, None)
                .unwrap_err();
        assert!(err.contains("crc mismatch"), "unexpected error: {err}");
    }

    #[test]
    fn newer_schema_archive_is_refused() {
        let fx = build_fixture();
        let validated =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap();
        let mut manifest = validated.manifest.clone();
        manifest.schema_version =
            crate::infrastructure::persistence::current_schema_version() + 1;
        std::fs::remove_dir_all(&fx.staging).unwrap();

        let rebuilt = fx.archive.with_file_name("newer.ffbak.zip");
        rebuild_archive_with_manifest(&fx.archive, &manifest, &rebuilt);
        let err = validate_archive_at(&rebuilt, &fx.staging, &fx.pending, None)
            .unwrap_err();
        assert!(err.contains("update FlowFlow"), "unexpected error: {err}");
    }

    #[test]
    fn pre_v10_archive_is_refused() {
        let fx = build_fixture();
        let validated =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap();
        let mut manifest = validated.manifest.clone();
        manifest.schema_version = 9;
        std::fs::remove_dir_all(&fx.staging).unwrap();

        let rebuilt = fx.archive.with_file_name("old.ffbak.zip");
        rebuild_archive_with_manifest(&fx.archive, &manifest, &rebuilt);
        let err = validate_archive_at(&rebuilt, &fx.staging, &fx.pending, None)
            .unwrap_err();
        assert!(err.contains("v9"), "unexpected error: {err}");
    }

    #[test]
    fn tampered_device_id_is_refused() {
        let fx = build_fixture();
        let validated =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap();
        let mut manifest = validated.manifest.clone();
        manifest.device_id = "spoofed-lineage".to_string();
        std::fs::remove_dir_all(&fx.staging).unwrap();

        let rebuilt = fx.archive.with_file_name("spoofed.ffbak.zip");
        rebuild_archive_with_manifest(&fx.archive, &manifest, &rebuilt);
        let err = validate_archive_at(&rebuilt, &fx.staging, &fx.pending, None)
            .unwrap_err();
        assert!(err.contains("device_id"), "unexpected error: {err}");
    }

    #[test]
    fn stray_zip_entry_is_refused() {
        let fx = build_fixture();
        let raw = std::fs::File::open(&fx.archive).unwrap();
        let mut src = zip::ZipArchive::new(raw).unwrap();
        let rebuilt = fx.archive.with_file_name("stray.ffbak.zip");
        let out = std::fs::File::create(&rebuilt).unwrap();
        let mut writer = zip::ZipWriter::new(out);
        let options = zip::write::SimpleFileOptions::default();
        for i in 0..src.len() {
            let entry = src.by_index_raw(i).unwrap();
            writer.raw_copy_file(entry).unwrap();
        }
        use std::io::Write;
        writer.start_file("extra/payload.bin", options).unwrap();
        writer.write_all(b"sneaky").unwrap();
        writer.finish().unwrap();

        let err = validate_archive_at(&rebuilt, &fx.staging, &fx.pending, None)
            .unwrap_err();
        assert!(err.contains("unexpected file"), "unexpected error: {err}");
    }

    #[test]
    fn stage_import_writes_markers_into_staged_db_then_commits_rename() {
        let fx = build_fixture();
        let validated =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap();

        stage_import_at(&validated, &fx.pending).expect("stage import");

        assert!(!fx.staging.exists(), "staging renamed away");
        let pending_db = fx.pending.join(DB_ENTRY_PATH);
        assert!(pending_db.exists());
        assert!(fx.pending.join(MANIFEST_PATH).exists());
        assert!(fx.pending.join("audio").join("recording_300.wav").exists());

        let conn = open_read_only(&pending_db).unwrap();
        let pending_flag: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'sync_restored_pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_flag, "true");
        let floor: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'sync_restored_floor'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(floor.parse::<i64>().is_ok());
        assert_no_sidecars(&pending_db).unwrap();
    }

    #[test]
    fn second_import_is_refused_while_pending() {
        let fx = build_fixture();
        let validated =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap();
        stage_import_at(&validated, &fx.pending).unwrap();

        let err =
            validate_archive_at(&fx.archive, &fx.staging, &fx.pending, None)
                .unwrap_err();
        assert!(err.contains("already pending"), "unexpected error: {err}");
    }

    fn rebuild_archive_with_manifest(
        original: &Path,
        manifest: &Manifest,
        dest: &Path,
    ) {
        use std::io::Write;
        let raw = std::fs::File::open(original).unwrap();
        let mut src = zip::ZipArchive::new(raw).unwrap();
        let out = std::fs::File::create(dest).unwrap();
        let mut writer = zip::ZipWriter::new(out);
        let options = zip::write::SimpleFileOptions::default();
        for i in 0..src.len() {
            let entry = src.by_index_raw(i).unwrap();
            if entry.name() == MANIFEST_PATH {
                continue;
            }
            writer.raw_copy_file(entry).unwrap();
        }
        writer.start_file(MANIFEST_PATH, options).unwrap();
        writer
            .write_all(manifest.to_json().unwrap().as_bytes())
            .unwrap();
        writer.finish().unwrap();
    }
}

#[cfg(all(test, not(target_os = "ios")))]
mod swap_tests {
    use super::*;
    use crate::domain::note::NewTextNote;

    struct SwapFixture {
        _root: tempfile::TempDir,
        paths: RestorePaths,
        archive: PathBuf,
        staging: PathBuf,
    }

    fn build_swap_fixture() -> SwapFixture {
        let root = tempfile::tempdir().unwrap();
        let base = root.path();
        let data_dir = base.join("data");
        let audio_dir = base.join("audio");
        let vectordb_dir = base.join("vectordb");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&audio_dir).unwrap();
        std::fs::create_dir_all(&vectordb_dir).unwrap();
        std::fs::write(vectordb_dir.join("stale.lance"), b"stale vectors")
            .unwrap();

        let source_db = data_dir.join("flowflow.db");
        let db = Database::open_at(source_db.clone()).unwrap();
        let note = db
            .create_text_note(&NewTextNote {
                title: Some("old state".into()),
                content: "pre-restore note".into(),
                tags: vec![],
            })
            .unwrap();
        std::fs::write(audio_dir.join("recording_500.wav"), b"RIFForiginal500")
            .unwrap();
        db.add_audio(&note.id, "recording_500.wav", 1.0).unwrap();
        drop(db);

        let export_staging = base.join("export_staging");
        let snapshot =
            create_scrubbed_snapshot(&source_db, &export_staging).unwrap();
        let archive = base.join("backup.ffbak.zip");
        build_archive(&snapshot, &audio_dir, &archive).unwrap();

        SwapFixture {
            paths: RestorePaths {
                pending: base.join("pending_restore"),
                bak: base.join("restore_bak"),
                data_db: source_db,
                audio_dir,
                vectordb_dir,
                error_file: base.join("restore_error.txt"),
            },
            archive,
            staging: base.join("import_staging"),
            _root: root,
        }
    }

    fn stage(fx: &SwapFixture) {
        let validated = validate_archive_at(
            &fx.archive,
            &fx.staging,
            &fx.paths.pending,
            None,
        )
        .unwrap();
        stage_import_at(&validated, &fx.paths.pending).unwrap();
    }

    fn assert_committed(fx: &SwapFixture) {
        assert!(fx.paths.data_db.exists(), "data db present after commit");
        assert!(!fx.paths.pending.exists(), "pending dir gone after commit");
        assert!(!fx.paths.vectordb_dir.exists(), "vectordb purged at swap");
        let db = Database::open_at(fx.paths.data_db.clone()).unwrap();
        assert_eq!(
            db.get_setting("sync_restored_pending").as_deref(),
            Some("true"),
            "restored marker travels inside the staged db"
        );
        let notes = db.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].content, "pre-restore note");
    }

    #[test]
    fn no_pending_is_a_noop() {
        let fx = build_swap_fixture();
        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::None);
        assert!(fx.paths.data_db.exists());
        assert!(fx.paths.vectordb_dir.exists(), "noop must not purge");
    }

    #[test]
    fn full_swap_commits_and_keeps_old_db_in_bak() {
        let fx = build_swap_fixture();
        stage(&fx);
        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert_committed(&fx);
        assert!(
            fx.paths.bak.join("flowflow.db").exists(),
            "old db kept in restore_bak until next successful boot"
        );
    }

    #[test]
    fn swap_resumes_after_kill_between_setaside_and_commit() {
        let fx = build_swap_fixture();
        stage(&fx);
        std::fs::create_dir_all(&fx.paths.bak).unwrap();
        std::fs::rename(&fx.paths.data_db, fx.paths.bak.join("flowflow.db"))
            .unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert_committed(&fx);
    }

    #[test]
    fn swap_resumes_after_kill_after_vectordb_purge() {
        let fx = build_swap_fixture();
        stage(&fx);
        std::fs::remove_dir_all(&fx.paths.vectordb_dir).unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert_committed(&fx);
    }

    #[test]
    fn commit_leftover_pending_dir_is_cleaned() {
        let fx = build_swap_fixture();
        stage(&fx);
        let pending_db = fx.paths.pending.join(DB_ENTRY_PATH);
        std::fs::rename(&pending_db, &fx.paths.data_db).unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert!(!fx.paths.pending.exists());
        assert!(fx.paths.data_db.exists());
    }

    #[test]
    fn corrupted_staged_db_rolls_back_old_state_intact() {
        let fx = build_swap_fixture();
        stage(&fx);
        let pending_db = fx.paths.pending.join(DB_ENTRY_PATH);
        let mut bytes = std::fs::read(&pending_db).unwrap();
        let len = bytes.len();
        bytes[len / 2] ^= 0xFF;
        std::fs::write(&pending_db, &bytes).unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert!(matches!(outcome, RestoreOutcome::RolledBack { .. }));
        assert!(fx.paths.data_db.exists(), "old db survives the rollback");
        assert!(!fx.paths.pending.exists());
        assert!(!fx.paths.bak.exists());
        assert!(
            fx.paths.error_file.exists(),
            "rollback leaves an error note"
        );
        let db = Database::open_at(fx.paths.data_db.clone()).unwrap();
        assert_eq!(db.get_setting("sync_restored_pending"), None);
        assert_eq!(db.list_notes().unwrap().len(), 1);
        assert_eq!(
            std::fs::read(fx.paths.audio_dir.join("recording_500.wav"))
                .unwrap(),
            b"RIFForiginal500"
        );
    }

    #[test]
    fn wav_collision_sets_old_file_aside_and_rollback_restores_it() {
        let fx = build_swap_fixture();
        stage(&fx);
        std::fs::write(
            fx.paths.audio_dir.join("recording_500.wav"),
            b"RIFFdifferent-live-audio",
        )
        .unwrap();
        let pending_db = fx.paths.pending.join(DB_ENTRY_PATH);
        let mut bytes = std::fs::read(&pending_db).unwrap();
        let len = bytes.len();
        bytes[len / 2] ^= 0xFF;
        std::fs::write(&pending_db, &bytes).unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert!(matches!(outcome, RestoreOutcome::RolledBack { .. }));
        assert_eq!(
            std::fs::read(fx.paths.audio_dir.join("recording_500.wav"))
                .unwrap(),
            b"RIFFdifferent-live-audio",
            "collided live wav must be restored by the rollback"
        );
    }

    #[test]
    fn wav_collision_commits_with_archive_copy() {
        let fx = build_swap_fixture();
        stage(&fx);
        std::fs::write(
            fx.paths.audio_dir.join("recording_500.wav"),
            b"RIFFdifferent-live-audio",
        )
        .unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert_eq!(
            std::fs::read(fx.paths.audio_dir.join("recording_500.wav"))
                .unwrap(),
            b"RIFForiginal500",
            "archive wav wins at commit"
        );
        assert_eq!(
            std::fs::read(fx.paths.bak.join("audio").join("recording_500.wav"))
                .unwrap(),
            b"RIFFdifferent-live-audio",
            "collided live wav kept in restore_bak"
        );
    }

    #[test]
    fn virgin_device_restore_commits() {
        let fx = build_swap_fixture();
        stage(&fx);
        std::fs::remove_file(&fx.paths.data_db).unwrap();
        std::fs::remove_file(fx.paths.audio_dir.join("recording_500.wav"))
            .unwrap();

        let outcome = apply_pending_restore_at(&fx.paths).unwrap();
        assert_eq!(outcome, RestoreOutcome::Committed);
        assert!(fx.paths.data_db.exists());
        assert!(fx.paths.audio_dir.join("recording_500.wav").exists());
    }

    #[test]
    fn restore_bak_purged_only_at_second_successful_boot() {
        let fx = build_swap_fixture();
        stage(&fx);
        apply_pending_restore_at(&fx.paths).unwrap();
        assert!(fx.paths.bak.exists());

        finalize_restore_bak_at(&fx.paths.bak);
        assert!(fx.paths.bak.exists(), "first boot only stamps");
        assert!(fx.paths.bak.join(".boot_survived").exists());

        finalize_restore_bak_at(&fx.paths.bak);
        assert!(!fx.paths.bak.exists(), "second boot purges");
    }
}

#[cfg(test)]
mod scrub_const_tests {
    use crate::infrastructure::persistence::settings_repo::{
        is_excluded_from_backup, DEVICE_LOCAL_SETTINGS,
        DEVICE_LOCAL_SETTING_PREFIXES, SENSITIVE_SETTINGS,
        SENSITIVE_SETTING_PREFIXES,
    };

    #[test]
    fn sensitive_list_covers_all_known_secrets() {
        for key in [
            "openai_api_key",
            "anthropic_api_key",
            "soniox_api_key",
            "sync_static_privkey",
            "sync_static_pubkey",
        ] {
            assert!(SENSITIVE_SETTINGS.contains(&key), "missing {key}");
        }
        assert!(SENSITIVE_SETTING_PREFIXES.contains(&"sync_psk_"));
    }

    #[test]
    fn device_local_list_covers_restore_and_peer_state() {
        for key in
            ["ai_consent", "sync_restored_pending", "sync_restored_floor"]
        {
            assert!(DEVICE_LOCAL_SETTINGS.contains(&key), "missing {key}");
        }
        for prefix in [
            "sync_peer_addr_",
            "sync_peer_acked_by_",
            "sync_restored_done_",
        ] {
            assert!(
                DEVICE_LOCAL_SETTING_PREFIXES.contains(&prefix),
                "missing {prefix}"
            );
        }
    }

    #[test]
    fn exclusion_predicate_matches_exact_keys_and_prefixes() {
        assert!(is_excluded_from_backup("openai_api_key"));
        assert!(is_excluded_from_backup("sync_psk_some-device-uuid"));
        assert!(is_excluded_from_backup("sync_peer_addr_some-device-uuid"));
        assert!(is_excluded_from_backup("sync_restored_done_peer-1"));
        assert!(is_excluded_from_backup("sync_restored_pending"));
        assert!(!is_excluded_from_backup("sync_device_id"));
        assert!(!is_excluded_from_backup("language"));
        assert!(!is_excluded_from_backup("llm_provider"));
        assert!(!is_excluded_from_backup("rag_max_sources"));
    }
}
