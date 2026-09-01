mod manifest;

pub use manifest::{
    current_platform, excluded_settings_description, Counts, Manifest,
    ManifestEntry, ARCHIVE_EXTENSION, ARCHIVE_FORMAT, ARCHIVE_VERSION,
    AUDIO_DIR_PREFIX, DB_ENTRY_PATH, MANIFEST_PATH, MIN_SCHEMA_VERSION,
};

mod archive;
mod export;
mod fs_util;
mod paths;
mod snapshot;
mod snapshot_db;
mod stage;
mod swap;
mod validate;

pub use archive::{archive_filename, build_archive};
pub use export::*;
pub use fs_util::{assert_no_sidecars, SIDECAR_SUFFIXES};
pub use paths::{
    activate_restore_lock, import_staging_dir, pending_restore_dir,
    restore_bak_dir, restore_lock_active, RestoreState, RESTORE_STATE_PATH,
};
pub use snapshot::{
    create_scrubbed_snapshot, ensure_chunks_backfilled, ScrubbedSnapshot,
};
pub use snapshot_db::{
    audio_paths_from_snapshot, open_read_only, snapshot_counts,
};
pub use stage::{stage_import, stage_import_at};
pub use swap::{
    apply_pending_restore_at, apply_pending_restore_or_abort,
    default_restore_paths, finalize_restore_bak, finalize_restore_bak_at,
    restore_recovery_window_active, take_restore_error, RestoreOutcome,
    RestorePaths,
};
pub use validate::{
    validate_archive, validate_archive_at, validate_staged_db, ValidatedImport,
};
