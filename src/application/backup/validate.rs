use std::path::{Path, PathBuf};

use super::fs_util::crc32_of_file;
use super::{
    assert_no_sidecars, audio_paths_from_snapshot, import_staging_dir,
    open_read_only, pending_restore_dir, snapshot_counts, Manifest,
    ARCHIVE_FORMAT, ARCHIVE_VERSION, AUDIO_DIR_PREFIX, DB_ENTRY_PATH,
    MANIFEST_PATH, MIN_SCHEMA_VERSION,
};

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

pub fn validate_staged_db(
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
