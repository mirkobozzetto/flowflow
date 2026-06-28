use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::fs_util::{crc32_of_file, fsync_dir, fsync_file, sidecar_paths};
use super::paths::restore_state_parent;
use super::{
    assert_no_sidecars, pending_restore_dir, restore_bak_dir, Manifest,
    RestoreState, AUDIO_DIR_PREFIX, DB_ENTRY_PATH, MANIFEST_PATH,
    RESTORE_STATE_PATH,
};

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
