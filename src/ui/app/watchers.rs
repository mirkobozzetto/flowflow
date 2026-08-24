use crate::application::note_persistence::append_transcription_to_note;
use crate::application::transcription_manager::{
    JobStatus, TranscriptionManager,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

pub fn use_transcription_watcher(
    manager: TranscriptionManager,
    db: Signal<Arc<Database>>,
    engine: Signal<Arc<SyncEngine>>,
    app: AppState,
) {
    use_future(move || {
        let manager = manager.clone();
        let db = db();
        let mut app = app;
        async move {
            loop {
                let snap = manager.snapshot();
                if *app.transcription_jobs.peek() != snap {
                    app.transcription_jobs.set(snap.clone());
                }
                let current = (app.current_note_id)();
                let viewing_note =
                    matches!((app.view)(), View::NoteDetail { .. });
                for (note_id, q) in snap.iter() {
                    let is_done = matches!(
                        q.front().map(|j| &j.status),
                        Some(JobStatus::Done(_))
                    );
                    let is_open = viewing_note
                        && current.as_deref() == Some(note_id.as_str());
                    if !is_done || is_open {
                        continue;
                    }
                    let Some((transcript, audio_id)) =
                        manager.take_done(note_id)
                    else {
                        continue;
                    };
                    if let Some(aid) = &audio_id {
                        let _ = db.set_audio_transcript(aid, &transcript);
                    }
                    append_transcription_to_note(
                        &db,
                        note_id,
                        &transcript.text(),
                    );
                    app.notes_version.set((app.notes_version)() + 1);
                    app.transcription_done_badge
                        .set((app.transcription_done_badge)() + 1);
                    engine.peek().schedule_debounced();
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    700,
                ))
                .await;
            }
        }
    });
}

pub fn use_sync_watcher(
    engine: Signal<Arc<SyncEngine>>,
    app: AppState,
    mut restore_locked: Signal<bool>,
    mut index_rebuilding: Signal<bool>,
) {
    use_future(move || {
        let mut app = app;
        async move {
            let mut last_seen = engine.peek().data_version();
            loop {
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    400,
                ))
                .await;
                let lock_now =
                    crate::application::backup::restore_lock_active();
                if *restore_locked.peek() != lock_now {
                    restore_locked.set(lock_now);
                }
                let rebuilding =
                    crate::application::backup::restore_recovery_window_active()
                        && crate::infrastructure::sync::reconcile::reconcile_running(
                        );
                if *index_rebuilding.peek() != rebuilding {
                    index_rebuilding.set(rebuilding);
                }
                let current_view = (app.view)();
                if crate::ui::sync::pending_pairing_uri_exists()
                    && current_view != View::SyncPairing
                {
                    app.previous_view.set(Some(current_view));
                    app.view.set(View::SyncPairing);
                }
                let current = engine.peek().data_version();
                if current == last_seen {
                    continue;
                }
                last_seen = current;
                app.sync_data_version.set(current);
                app.notes_version.set((app.notes_version)() + 1);
                app.folders_version.set((app.folders_version)() + 1);
                app.attachments_version.set((app.attachments_version)() + 1);
            }
        }
    });
}

/// One-gesture capture: the Control Center / Lock Screen / Action Button
/// control opens the app on flowflow://record; this watcher drains that
/// deep link and starts a new voice recording as if the mic button was
/// tapped. Only from a quiet state - never steals an ongoing recording.
pub fn use_record_deeplink_watcher(
    app: AppState,
    recorder: Signal<
        Arc<std::sync::Mutex<crate::infrastructure::audio::AudioRecorder>>,
    >,
) {
    use crate::infrastructure::audio::RecordingState;
    use_future(move || {
        let mut app = app;
        async move {
            loop {
                #[cfg(target_os = "ios")]
                let group_flag =
                    crate::infrastructure::platform::ios::take_pending_record();
                #[cfg(not(target_os = "ios"))]
                let group_flag = false;
                if group_flag
                    || crate::infrastructure::sync::deeplink::take_matching(
                        "flowflow://record",
                    )
                    .is_some()
                {
                    let state = (app.recording_state)();
                    let quiet = state == RecordingState::Idle
                        || matches!(state, RecordingState::Error(_))
                        || matches!(state, RecordingState::Transcribed { .. });
                    if quiet
                        && !crate::application::backup::restore_lock_active()
                    {
                        app.current_note_id.set(None);
                        crate::ui::recording::start_recording(recorder, app);
                        // Empty-id NoteDetail = the new-note composer: the
                        // send button turns the take into a note and the
                        // user STAYS on it (not dumped on the list). Bounce
                        // through NotesList first: a NoteDetail already on
                        // screen would otherwise keep its mounted state.
                        app.view.set(View::NotesList);
                        futures_timer::Delay::new(
                            std::time::Duration::from_millis(30),
                        )
                        .await;
                        app.view.set(View::NoteDetail {
                            note_id: String::new(),
                        });
                    }
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    300,
                ))
                .await;
            }
        }
    });
}

/// A tapped share link (flowflow://share/{code}) opens the read-only view.
/// Same mailbox as the record deep link, scoped by prefix.
pub fn use_share_deeplink_watcher(app: AppState) {
    use crate::domain::share::{parse_share_link, SHARE_LINK_PREFIX};
    use_future(move || {
        let mut app = app;
        async move {
            loop {
                if let Some(uri) =
                    crate::infrastructure::sync::deeplink::take_matching(
                        SHARE_LINK_PREFIX,
                    )
                {
                    if let Some(code) = parse_share_link(&uri) {
                        app.previous_view.set(Some(View::NotesList));
                        app.view.set(View::SharedView {
                            code: code.to_string(),
                        });
                    }
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    500,
                ))
                .await;
            }
        }
    });
}

/// One deletion-alignment pass at boot (proposal 0001, lifecycle rule 3):
/// kept notes whose author deleted the original are removed (note +
/// embeddings); dead shares grey their provenance out. Cheap when the device
/// holds no kept content; network errors change nothing.
pub fn use_share_align_watcher(app: AppState, db: Signal<Arc<Database>>) {
    use_future(move || {
        let mut app = app;
        async move {
            let database = db();
            if database.all_provenance_codes().is_empty() {
                return;
            }
            let events =
                crate::application::sharing::align_kept_content(&database)
                    .await;
            if !events.is_empty() {
                app.notes_version.set((app.notes_version)() + 1);
            }
        }
    });
}

/// Drain the share-extension inbox (app group): shared text/URLs become
/// notes, shared documents ride the attachment pipeline. Cheap poll - the
/// directory is empty or absent almost always.
#[allow(unused_variables)]
pub fn use_share_inbox_watcher(app: AppState, db: Signal<Arc<Database>>) {
    #[cfg(target_os = "ios")]
    use_future(move || {
        let mut app = app;
        async move {
            loop {
                let inbox =
                    crate::infrastructure::platform::ios::app_group_inbox_dir();
                if let Some(inbox) = inbox {
                    let pending = std::fs::read_dir(&inbox)
                        .map(|mut d| {
                            d.any(|e| {
                                e.ok()
                                    .map(|e| {
                                        e.path()
                                            .extension()
                                            .and_then(|x| x.to_str())
                                            == Some("json")
                                    })
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false);
                    if pending {
                        let db = db();
                        let n =
                            crate::application::share_inbox::drain(&db, &inbox)
                                .await;
                        if n > 0 {
                            app.notes_version.set((app.notes_version)() + 1);
                            app.attachments_version
                                .set((app.attachments_version)() + 1);
                        }
                    }
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    2000,
                ))
                .await;
            }
        }
    });
}

/// Keep joined spaces fresh (proposal 0002 T13). The 30 s floor lives in
/// `pull_all_due`, so this loop only decides how often to ASK; a device with no
/// space does no work at all, and a pull with nothing to report costs one round
/// trip, not a transfer.
pub fn use_space_pull_watcher(app: AppState, db: Signal<Arc<Database>>) {
    use_future(move || {
        let mut app = app;
        async move {
            loop {
                let database = db();
                if !database.list_spaces().unwrap_or_default().is_empty() {
                    let changed =
                        crate::application::space::pull_all_due(&database)
                            .await;
                    if changed > 0 {
                        app.notes_version.set((app.notes_version)() + 1);
                        app.folders_version.set((app.folders_version)() + 1);
                    }
                }
                futures_timer::Delay::new(std::time::Duration::from_secs(30))
                    .await;
            }
        }
    });
}

pub fn use_picker_reset_on_view(app: AppState) {
    use_effect(move || {
        let _ = (app.view)();
        let mut app = app;
        app.show_folder_picker.set(false);
    });
}

pub fn use_history_tracker(app: AppState) {
    let mut last_view = use_signal(|| app.view.peek().clone());
    use_effect(move || {
        let v = (app.view)();
        let prev = last_view.peek().clone();
        if v == prev {
            return;
        }
        last_view.set(v);
        let mut app = app;
        if *app.history_nav.peek() {
            app.history_nav.set(false);
            return;
        }
        let mut history = app.view_history.write();
        history.push(prev);
        if history.len() > 20 {
            history.remove(0);
        }
        drop(history);
        app.view_future.write().clear();
    });
}
