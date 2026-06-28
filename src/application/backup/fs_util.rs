use std::path::{Path, PathBuf};

pub const SIDECAR_SUFFIXES: &[&str] = &["-wal", "-shm", "-journal"];

pub(crate) fn sidecar_paths(db_file: &Path) -> Vec<PathBuf> {
    SIDECAR_SUFFIXES
        .iter()
        .map(|suffix| {
            let mut name = db_file.as_os_str().to_os_string();
            name.push(suffix);
            PathBuf::from(name)
        })
        .collect()
}

pub fn assert_no_sidecars(db_file: &Path) -> Result<(), String> {
    for sidecar in sidecar_paths(db_file) {
        if sidecar.exists() {
            return Err(format!(
                "sidecar must not exist: {}",
                sidecar.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn fsync_file(path: &Path) -> Result<(), String> {
    std::fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("fsync {}: {e}", path.display()))
}

pub(crate) fn fsync_dir(path: &Path) -> Result<(), String> {
    std::fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("fsync dir {}: {e}", path.display()))
}

pub(crate) fn crc32_of_file(path: &Path) -> Result<u32, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("crc open {}: {e}", path.display()))?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("crc read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}
