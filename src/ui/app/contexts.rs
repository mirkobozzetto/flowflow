use super::boot;
use crate::infrastructure::audio::AudioRecorder;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::ui::transcription_manager::TranscriptionManager;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

pub struct AppContexts {
    pub db: Signal<Arc<Database>>,
    pub engine: Signal<Arc<SyncEngine>>,
    pub recorder: Signal<Arc<Mutex<AudioRecorder>>>,
    pub manager: TranscriptionManager,
    pub restore_locked: Signal<bool>,
    pub index_rebuilding: Signal<bool>,
}

pub fn use_app_contexts() -> AppContexts {
    let db = use_context_provider(|| {
        let db = Arc::new(Database::open().expect("Failed to open database"));
        boot::run_boot_effects(&db);
        Signal::new(db)
    });

    let restore_locked =
        use_signal(crate::application::backup::restore_lock_active);
    let index_rebuilding = use_signal(|| false);

    let engine: Signal<Arc<SyncEngine>> =
        use_context_provider(|| Signal::new(SyncEngine::start(db())));

    let recorder: Signal<Arc<Mutex<AudioRecorder>>> =
        use_context_provider(|| {
            Signal::new(Arc::new(Mutex::new(AudioRecorder::new())))
        });

    let manager = use_context_provider(|| {
        let m = TranscriptionManager::new(db());
        m.resume_pending();
        m
    });

    AppContexts {
        db,
        engine,
        recorder,
        manager,
        restore_locked,
        index_rebuilding,
    }
}
