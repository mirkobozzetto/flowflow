use crate::services::constants::{WhisperCatalogEntry, WHISPER_CATALOG};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static DOWNLOAD_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn catalog() -> &'static [WhisperCatalogEntry] {
    WHISPER_CATALOG
}

pub fn entry(id: &str) -> Option<&'static WhisperCatalogEntry> {
    WHISPER_CATALOG.iter().find(|e| e.id == id)
}

pub fn models_dir() -> PathBuf {
    #[cfg(target_os = "ios")]
    let base = crate::platform::ios::documents_dir();
    #[cfg(not(target_os = "ios"))]
    let base = crate::db::desktop_data_dir();
    base.join(crate::services::constants::WHISPER_MODELS_SUBDIR)
}

pub fn model_path(dir: &Path, id: &str) -> Option<PathBuf> {
    entry(id).map(|e| dir.join(e.filename))
}

pub fn is_downloaded(dir: &Path, id: &str) -> bool {
    model_path(dir, id).map(|p| p.is_file()).unwrap_or(false)
}

pub fn list_downloaded(dir: &Path) -> Vec<&'static str> {
    WHISPER_CATALOG
        .iter()
        .filter(|e| dir.join(e.filename).is_file())
        .map(|e| e.id)
        .collect()
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let path =
        model_path(dir, id).ok_or_else(|| format!("Modèle inconnu: {id}"))?;
    if path.is_file() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Suppression modèle: {e}"))?;
    }
    let _ = std::fs::remove_file(part_path(&path));
    Ok(())
}

fn part_path(target: &Path) -> PathBuf {
    let mut os = target.as_os_str().to_owned();
    os.push(".part");
    PathBuf::from(os)
}

pub fn check_disk_guardrail(
    available: u64,
    model_bytes: u64,
) -> Result<(), String> {
    let needed = model_bytes.saturating_mul(2);
    if available < needed {
        return Err(format!(
            "Espace disque insuffisant: {} Mo libres, {} Mo requis",
            available / 1_048_576,
            needed / 1_048_576
        ));
    }
    Ok(())
}

struct DownloadSlot;

impl Drop for DownloadSlot {
    fn drop(&mut self) {
        DOWNLOAD_ACTIVE.store(false, Ordering::SeqCst);
    }
}

fn acquire_download_slot() -> Result<DownloadSlot, String> {
    if DOWNLOAD_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Un téléchargement est déjà en cours".to_string());
    }
    Ok(DownloadSlot)
}

pub async fn download(
    dir: &Path,
    id: &str,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let entry = entry(id).ok_or_else(|| format!("Modèle inconnu: {id}"))?;
    let _slot = acquire_download_slot()?;
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Dossier modèles: {e}"))?;
    let available =
        fs4::available_space(dir).map_err(|e| format!("Espace disque: {e}"))?;
    check_disk_guardrail(available, entry.bytes)?;
    let target = dir.join(entry.filename);
    let part = part_path(&target);
    if let Err(e) = stream_to_part(entry, &part, &mut progress).await {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    let verify_part = part.clone();
    let verify = tokio::task::spawn_blocking(move || {
        verify_file(&verify_part, entry.bytes, entry.sha256)
    })
    .await
    .map_err(|e| format!("Vérification: {e}"))?;
    if let Err(e) = verify {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, &target)
        .map_err(|e| format!("Finalisation modèle: {e}"))?;
    Ok(())
}

async fn stream_to_part(
    entry: &WhisperCatalogEntry,
    part: &Path,
    progress: &mut impl FnMut(u64, u64),
) -> Result<(), String> {
    use futures::StreamExt;
    use std::io::Write;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client HTTP: {e}"))?;
    let resp = client
        .get(entry.url)
        .send()
        .await
        .map_err(|e| format!("Téléchargement: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement échoué ({})", resp.status()));
    }
    let total = resp.content_length().unwrap_or(entry.bytes);
    let mut file = std::fs::File::create(part)
        .map_err(|e| format!("Fichier temporaire: {e}"))?;
    let stream = resp.bytes_stream();
    tokio::pin!(stream);
    let mut done: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Flux interrompu: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Écriture modèle: {e}"))?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(|e| format!("Écriture modèle: {e}"))?;
    Ok(())
}

pub fn verify_file(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Vérification taille: {e}"))?;
    if meta.len() != expected_bytes {
        return Err(format!(
            "Taille inattendue: {} octets au lieu de {expected_bytes}",
            meta.len()
        ));
    }
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Vérification sha256: {e}"))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|e| format!("Vérification sha256: {e}"))?;
    let digest = format!("{:x}", hasher.finalize());
    if digest != expected_sha256 {
        return Err("Empreinte sha256 invalide".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_five_models_with_pinned_hashes() {
        assert_eq!(catalog().len(), 5);
        for e in catalog() {
            assert_eq!(e.sha256.len(), 64, "{} sha256 must be pinned", e.id);
            assert!(e.bytes > 0);
            assert!(e.url.starts_with("https://"));
        }
        assert!(entry("large-v3-turbo-q5_0").is_some());
    }

    #[test]
    fn unknown_model_is_rejected() {
        assert!(entry("gpt-5").is_none());
        let dir = tempfile::tempdir().unwrap();
        assert!(model_path(dir.path(), "gpt-5").is_none());
        assert!(delete(dir.path(), "gpt-5").is_err());
    }

    #[test]
    fn list_and_delete_roundtrip_on_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_downloaded(dir.path()).is_empty());
        assert!(!is_downloaded(dir.path(), "tiny"));

        let path = model_path(dir.path(), "tiny").unwrap();
        std::fs::write(&path, b"fake model").unwrap();
        assert!(is_downloaded(dir.path(), "tiny"));
        assert_eq!(list_downloaded(dir.path()), vec!["tiny"]);

        delete(dir.path(), "tiny").unwrap();
        assert!(!is_downloaded(dir.path(), "tiny"));
        assert!(list_downloaded(dir.path()).is_empty());
    }

    #[test]
    fn delete_removes_orphan_part_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = model_path(dir.path(), "base").unwrap();
        let part = part_path(&path);
        std::fs::write(&part, b"partial").unwrap();
        delete(dir.path(), "base").unwrap();
        assert!(!part.exists());
    }

    #[test]
    fn disk_guardrail_requires_twice_the_size() {
        assert!(check_disk_guardrail(100, 50).is_ok());
        assert!(check_disk_guardrail(99, 50).is_err());
        assert!(check_disk_guardrail(0, u64::MAX).is_err());
    }

    #[test]
    fn verify_file_checks_size_then_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.bin");
        std::fs::write(&path, b"hello").unwrap();

        let good_sha =
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_file(&path, 5, good_sha).is_ok());
        assert!(verify_file(&path, 4, good_sha).is_err());
        let bad_sha =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_file(&path, 5, bad_sha).is_err());
    }

    #[test]
    fn single_download_slot() {
        let slot = acquire_download_slot().unwrap();
        assert!(acquire_download_slot().is_err());
        drop(slot);
        assert!(acquire_download_slot().is_ok());
    }
}
