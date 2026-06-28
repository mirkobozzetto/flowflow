use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::DB_ENTRY_PATH;

static RESTORE_LOCK: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn activate_restore_lock() {
    RESTORE_LOCK.store(true, std::sync::atomic::Ordering::SeqCst);
    eprintln!("[backup] restore lock active: restart required");
}

pub fn restore_lock_active() -> bool {
    RESTORE_LOCK.load(std::sync::atomic::Ordering::SeqCst)
        || pending_restore_dir().join(DB_ENTRY_PATH).exists()
}

pub(crate) fn restore_state_parent() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        let home = std::env::var("HOME").expect("HOME not set on iOS");
        PathBuf::from(home).join("Library/Application Support")
    }
    #[cfg(not(target_os = "ios"))]
    {
        crate::infrastructure::persistence::desktop_data_dir()
    }
}

pub fn pending_restore_dir() -> PathBuf {
    restore_state_parent().join("pending_restore")
}

pub fn import_staging_dir() -> PathBuf {
    restore_state_parent().join("import_staging")
}

pub fn restore_bak_dir() -> PathBuf {
    restore_state_parent().join("restore_bak")
}

pub const RESTORE_STATE_PATH: &str = "restore_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreState {
    pub staged_db_crc32: u32,
    pub floor: i64,
}
