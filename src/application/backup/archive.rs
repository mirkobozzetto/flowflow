use std::path::{Path, PathBuf};

use super::snapshot_db::snapshot_device_id;
use super::{
    audio_paths_from_snapshot, snapshot_counts, Manifest, ManifestEntry,
    ARCHIVE_EXTENSION, AUDIO_DIR_PREFIX, DB_ENTRY_PATH, MANIFEST_PATH,
};

fn zip_file_entry(
    zip: &mut zip::ZipWriter<std::fs::File>,
    options: zip::write::SimpleFileOptions,
    entry_path: &str,
    source: &Path,
) -> Result<u32, String> {
    use std::io::{Read, Write};
    let mut input = std::fs::File::open(source)
        .map_err(|e| format!("zip open {}: {e}", source.display()))?;
    zip.start_file(entry_path, options)
        .map_err(|e| format!("zip start {entry_path}: {e}"))?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = input
            .read(&mut buf)
            .map_err(|e| format!("zip read {entry_path}: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        zip.write_all(&buf[..n])
            .map_err(|e| format!("zip write {entry_path}: {e}"))?;
    }
    Ok(hasher.finalize())
}

pub fn build_archive(
    snapshot: &Path,
    audio_dir: &Path,
    archive_path: &Path,
) -> Result<Manifest, String> {
    use std::io::Write;

    let device_id = snapshot_device_id(snapshot)?;
    let counts = snapshot_counts(snapshot)?;
    let mut manifest = Manifest::new(device_id, counts);

    let mut wav_sources: Vec<(String, PathBuf)> = Vec::new();
    for filename in audio_paths_from_snapshot(snapshot)? {
        let source = audio_dir.join(&filename);
        if source.is_file() {
            wav_sources.push((filename, source));
        } else {
            eprintln!(
                "[backup] audio missing, recorded in manifest: {filename}"
            );
            manifest.audio_missing.push(filename);
        }
    }

    let file = std::fs::File::create(archive_path)
        .map_err(|e| format!("archive create: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let db_crc = zip_file_entry(&mut zip, options, DB_ENTRY_PATH, snapshot)?;
    manifest.entries.push(ManifestEntry {
        path: DB_ENTRY_PATH.to_string(),
        crc32: db_crc,
    });
    for (filename, source) in &wav_sources {
        let entry_path = format!("{AUDIO_DIR_PREFIX}{filename}");
        let crc = zip_file_entry(&mut zip, options, &entry_path, source)?;
        manifest.entries.push(ManifestEntry {
            path: entry_path,
            crc32: crc,
        });
    }

    let manifest_json = manifest.to_json()?;
    zip.start_file(MANIFEST_PATH, options)
        .map_err(|e| format!("zip start manifest: {e}"))?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| format!("zip write manifest: {e}"))?;
    zip.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(manifest)
}

pub fn archive_filename() -> String {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    format!("flowflow-backup-{stamp}.{ARCHIVE_EXTENSION}")
}
