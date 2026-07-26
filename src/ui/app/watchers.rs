use crate::application::transcription_manager::{
    append_transcription_to_note, JobStatus, TranscriptionManager,
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
                    if is_done && !is_open {
                        if let Some(transcript) = manager.take_done(note_id) {
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
                    }
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
