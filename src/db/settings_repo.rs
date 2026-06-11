use crate::db::Database;

pub const LANGUAGE_KEY: &str = "language";

pub const SENSITIVE_SETTINGS: &[&str] = &[
    "openai_api_key",
    "anthropic_api_key",
    "soniox_api_key",
    "sync_static_privkey",
    "sync_static_pubkey",
];

pub const SENSITIVE_SETTING_PREFIXES: &[&str] = &["sync_psk_"];

pub const DEVICE_LOCAL_SETTINGS: &[&str] =
    &["ai_consent", "sync_restored_pending", "sync_restored_floor"];

pub const DEVICE_LOCAL_SETTING_PREFIXES: &[&str] = &[
    "sync_peer_addr_",
    "sync_peer_acked_by_",
    "sync_restored_done_",
    "sync_rebind_",
];

pub fn is_excluded_from_backup(key: &str) -> bool {
    SENSITIVE_SETTINGS.contains(&key)
        || DEVICE_LOCAL_SETTINGS.contains(&key)
        || SENSITIVE_SETTING_PREFIXES
            .iter()
            .any(|p| key.starts_with(p))
        || DEVICE_LOCAL_SETTING_PREFIXES
            .iter()
            .any(|p| key.starts_with(p))
}

impl Database {
    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )
        .map_err(|e| format!("Set setting: {e}"))?;
        Ok(())
    }
}
