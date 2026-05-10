use crate::services::audio::RecordingState;
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    NotesList,
    NoteDetail { note_id: String },
}

#[derive(Clone)]
pub struct AppState {
    pub view: Signal<View>,
    pub sidebar_open: Signal<bool>,
    pub selected_folder_id: Signal<Option<String>>,
    pub recording_state: Signal<RecordingState>,
    pub folders_version: Signal<u32>,
    pub sliding_out: Signal<bool>,
    pub audio_levels: Signal<Vec<f32>>,
    pub notes_version: Signal<u32>,
}
