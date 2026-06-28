#[cfg(not(target_os = "ios"))]
use std::path::Path;
use std::path::PathBuf;

use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::VectorStore;

use super::{
    archive_filename, build_archive, create_scrubbed_snapshot,
    ensure_chunks_backfilled,
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
