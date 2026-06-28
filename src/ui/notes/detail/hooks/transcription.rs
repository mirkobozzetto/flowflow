use crate::application::note_persistence::persist_last_transcription;
use crate::application::transcription_manager::TranscriptionManager;
use crate::infrastructure::audio::RecordingState;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

pub fn use_transcription_sink(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    manager: TranscriptionManager,
    mut content: Signal<String>,
    local_note_id: Signal<String>,
    mut audios_version: Signal<u32>,
) {
    use_effect(move || {
        if let RecordingState::Transcribed(text) = (app.recording_state)() {
            let current = content();
            if current.is_empty() {
                content.set(text.clone());
            } else {
                content.set(format!("{}\n{}", current, text));
            }
            let id = local_note_id();
            if !id.is_empty() && persist_last_transcription(&db(), &id, &text) {
                audios_version.set(audios_version() + 1);
            }
            app.recording_state.set(RecordingState::Idle);
        }
    });

    let observe_manager = manager.clone();
    use_effect(move || {
        let _ = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() {
            return;
        }
        if let Some(text) = observe_manager.take_done(&nid) {
            let cur = content.peek().clone();
            if cur.is_empty() {
                content.set(text);
            } else {
                content.set(format!("{cur}\n{text}"));
            }
            app.transcription_jobs.set(observe_manager.snapshot());
        }
    });

    use_effect(move || {
        app.transcription_done_badge.set(0);
    });
}
