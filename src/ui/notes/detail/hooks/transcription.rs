use crate::application::embed::embed_note;
use crate::application::note_persistence::commit_transcription;
use crate::application::transcription_manager::TranscriptionManager;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn use_transcription_sink(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    engine: Signal<Arc<SyncEngine>>,
    manager: TranscriptionManager,
    mut content: Signal<String>,
    mut base_content: Signal<String>,
    local_note_id: Signal<String>,
    mut audios_version: Signal<u32>,
) {
    let observe_manager = manager.clone();
    use_effect(move || {
        let _ = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() {
            return;
        }
        let Some((transcript, audio_id)) = observe_manager.take_done(&nid)
        else {
            return;
        };
        // A recording anchors its word timings to its own clip; an import has
        // no `note_audios` row, so only its text is kept.
        if let Some(aid) = &audio_id {
            let _ = db().set_audio_transcript(aid, &transcript);
            audios_version.set(audios_version() + 1);
        }
        let current = content.peek().clone();
        if let Some(c) =
            commit_transcription(&db(), &nid, &current, &transcript.text())
        {
            content.set(c.merged.clone());
            base_content.set(c.merged.clone());
            embed_note(nid.clone(), c.title, c.merged, c.tags, c.created_at);
            app.notes_version.set((app.notes_version)() + 1);
            engine.peek().schedule_debounced();
        }
        app.transcription_jobs.set(observe_manager.snapshot());
    });

    use_effect(move || {
        app.transcription_done_badge.set(0);
    });
}
