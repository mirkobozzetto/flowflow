use crate::infrastructure::persistence::Database;

pub fn run_boot_effects(db: &std::sync::Arc<Database>) {
    if crate::application::backup::restore_recovery_window_active() {
        eprintln!("[backup] orphan audio cleanup skipped (recovery window)");
    } else {
        db.cleanup_orphan_audio(&crate::infrastructure::audio::output_dir());
    }
    crate::application::backup::finalize_restore_bak();
    crate::infrastructure::sync::reconcile::run_boot_reconcile();
    // Warm the plan cache off the boot path so the sync Hello can advertise
    // premium (RFC 0026) even before the account page was ever opened.
    let heal_db = db.clone();
    std::thread::spawn(move || {
        crate::application::account_heal::refresh_account_cache(&heal_db);
    });
    #[cfg(target_os = "ios")]
    {
        crate::infrastructure::platform::ios::sync_ffi::observe_background_checkpoint();
        crate::infrastructure::platform::ios::sync_ffi::observe_restore_foreground();
    }
}

pub fn load_consent(db: &Database) -> Option<bool> {
    if option_env!("FLOWFLOW_RESET_CONSENT") == Some("1") {
        let _ = db.set_setting("ai_consent", "");
    }
    db.get_setting("ai_consent").map(|v| v == "true")
}

pub fn load_lang(db: &Database) -> String {
    db.get_setting(
        crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY,
    )
    .unwrap_or_else(crate::infrastructure::platform::detect_system_language)
}
