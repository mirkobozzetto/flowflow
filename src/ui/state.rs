use crate::application::transcription_manager::Job;
use crate::domain::Attachment;
use crate::domain::ChatScope;
use crate::infrastructure::audio::RecordingState;
use dioxus::prelude::*;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub enum SidebarTab {
    Notes,
    Chats,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsSection {
    General,
    Account,
    Intelligence,
    Transcription,
    Storage,
    Backup,
    Connections,
    Privacy,
    Shortcuts,
}

impl SettingsSection {
    pub fn title_key(self) -> &'static str {
        match self {
            Self::General => "settings-section-general",
            Self::Account => "settings-section-account",
            Self::Intelligence => "settings-section-intelligence",
            Self::Transcription => "settings-section-transcription",
            Self::Storage => "settings-storage-title",
            Self::Backup => "settings-backup-title",
            Self::Connections => "settings-section-connections",
            Self::Privacy => "settings-section-privacy",
            Self::Shortcuts => "settings-section-shortcuts",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    NotesList,
    NoteDetail { note_id: String },
    ThreadDetail { thread_id: String },
    Chat { conversation_id: Option<String> },
    Settings,
    SettingsSection(SettingsSection),
    SyncPairing,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub view: Signal<View>,
    pub sidebar_open: Signal<bool>,
    pub selected_folder_id: Signal<Option<String>>,
    pub recording_state: Signal<RecordingState>,
    pub folders_version: Signal<u32>,
    pub sliding_out: Signal<bool>,
    pub audio_levels: Signal<Vec<f32>>,
    pub notes_version: Signal<u32>,
    pub current_note_id: Signal<Option<String>>,
    pub previous_view: Signal<Option<View>>,
    pub search_query: Signal<String>,
    pub show_note_menu: Signal<bool>,
    pub attachments_version: Signal<u32>,
    pub attachment_modal: Signal<Option<Attachment>>,
    pub show_chat_menu: Signal<bool>,
    pub show_thread_menu: Signal<bool>,
    pub show_tools_menu: Signal<bool>,
    pub show_mention_menu: Signal<bool>,
    // Per-chat web-search toggle (mirrors chat_scope: restored on open, persisted
    // per conversation). Default OFF; the composer "+" menu flips it.
    pub chat_web: Signal<bool>,
    pub sidebar_tab: Signal<SidebarTab>,
    pub show_folder_picker: Signal<bool>,
    pub chat_scope: Signal<Option<ChatScope>>,
    // Scope a brand-new chat should adopt (e.g. opened from a folder); consumed
    // and cleared by the chat view when it creates the conversation.
    pub pending_chat_scope: Signal<Option<ChatScope>>,
    pub detail_folder_id: Signal<Option<String>>,
    pub ai_consent: Signal<Option<bool>>,
    pub current_lang: Signal<String>,
    pub transcription_jobs: Signal<HashMap<String, VecDeque<Job>>>,
    pub transcription_done_badge: Signal<usize>,
    pub audio_import_requested: Signal<bool>,
    pub sync_data_version: Signal<u64>,
    pub view_history: Signal<Vec<View>>,
    pub view_future: Signal<Vec<View>>,
    pub history_nav: Signal<bool>,
    pub picker_kb_up: Signal<u32>,
    pub picker_kb_down: Signal<u32>,
    pub picker_kb_commit: Signal<u32>,
}

impl AppState {
    pub fn new(consent: Option<bool>, lang: String) -> Self {
        Self {
            view: Signal::new(View::NotesList),
            sidebar_open: Signal::new(false),
            selected_folder_id: Signal::new(None),
            recording_state: Signal::new(RecordingState::Idle),
            folders_version: Signal::new(0),
            sliding_out: Signal::new(false),
            audio_levels: Signal::new(vec![0.0; 12]),
            notes_version: Signal::new(0),
            current_note_id: Signal::new(None),
            previous_view: Signal::new(None),
            search_query: Signal::new(String::new()),
            show_note_menu: Signal::new(false),
            attachments_version: Signal::new(0),
            attachment_modal: Signal::new(None),
            show_chat_menu: Signal::new(false),
            show_thread_menu: Signal::new(false),
            show_tools_menu: Signal::new(false),
            show_mention_menu: Signal::new(false),
            chat_web: Signal::new(false),
            sidebar_tab: Signal::new(SidebarTab::Notes),
            show_folder_picker: Signal::new(false),
            chat_scope: Signal::new(None),
            pending_chat_scope: Signal::new(None),
            detail_folder_id: Signal::new(None),
            ai_consent: Signal::new(consent),
            current_lang: Signal::new(lang),
            transcription_jobs: Signal::new(HashMap::new()),
            transcription_done_badge: Signal::new(0),
            audio_import_requested: Signal::new(false),
            sync_data_version: Signal::new(0),
            view_history: Signal::new(Vec::new()),
            view_future: Signal::new(Vec::new()),
            history_nav: Signal::new(false),
            picker_kb_up: Signal::new(0),
            picker_kb_down: Signal::new(0),
            picker_kb_commit: Signal::new(0),
        }
    }
}
