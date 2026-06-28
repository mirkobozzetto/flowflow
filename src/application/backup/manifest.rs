use serde::{Deserialize, Serialize};

use crate::infrastructure::persistence::settings_repo::{
    DEVICE_LOCAL_SETTINGS, DEVICE_LOCAL_SETTING_PREFIXES, SENSITIVE_SETTINGS,
    SENSITIVE_SETTING_PREFIXES,
};

pub const ARCHIVE_FORMAT: &str = "flowflow-backup";
pub const ARCHIVE_VERSION: u32 = 1;
pub const MIN_SCHEMA_VERSION: i64 = 10;
pub const ARCHIVE_EXTENSION: &str = "ffbak.zip";
pub const MANIFEST_PATH: &str = "manifest.json";
pub const DB_ENTRY_PATH: &str = "db/flowflow.db";
pub const AUDIO_DIR_PREFIX: &str = "audio/";
pub const EXCLUDED_TABLES: &[&str] = &["sync_peers", "pending_transcriptions"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub archive_version: u32,
    pub schema_version: i64,
    pub app_version: String,
    pub platform: String,
    pub device_id: String,
    pub created_at: String,
    pub counts: Counts,
    pub audio_missing: Vec<String>,
    pub excluded_settings: Vec<String>,
    pub excluded_tables: Vec<String>,
    pub entries: Vec<ManifestEntry>,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default,
)]
pub struct Counts {
    pub notes: i64,
    pub folders: i64,
    pub threads: i64,
    pub attachments: i64,
    pub conversations: i64,
    pub audio_files: i64,
    pub chunks: i64,
    pub reminders: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: String,
    pub crc32: u32,
}

pub fn current_platform() -> &'static str {
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        "desktop"
    }
}

pub fn excluded_settings_description() -> Vec<String> {
    SENSITIVE_SETTINGS
        .iter()
        .chain(DEVICE_LOCAL_SETTINGS.iter())
        .map(|k| k.to_string())
        .chain(
            SENSITIVE_SETTING_PREFIXES
                .iter()
                .chain(DEVICE_LOCAL_SETTING_PREFIXES.iter())
                .map(|p| format!("{p}*")),
        )
        .collect()
}

impl Manifest {
    pub fn new(device_id: String, counts: Counts) -> Self {
        Self {
            format: ARCHIVE_FORMAT.to_string(),
            archive_version: ARCHIVE_VERSION,
            schema_version:
                crate::infrastructure::persistence::current_schema_version(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: current_platform().to_string(),
            device_id,
            created_at: crate::infrastructure::persistence::now_iso(),
            counts,
            audio_missing: Vec::new(),
            excluded_settings: excluded_settings_description(),
            excluded_tables: EXCLUDED_TABLES
                .iter()
                .map(|t| t.to_string())
                .collect(),
            entries: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("manifest serialize: {e}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("manifest parse: {e}"))
    }
}
