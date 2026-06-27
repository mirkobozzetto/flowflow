mod action_card;
mod app;
mod attachment_modal;
mod chat;
mod chat_input;
mod clipboard;
mod consent;
mod fab;
pub(crate) mod folder_picker;
pub mod icons;
mod keyboard;
pub(crate) mod kit;
mod note_card;
mod note_list;
mod notes;
mod recording;
mod restore_lock;
mod settings;
mod sidebar;
mod state;
mod sync;
mod thread;
mod top_bar;
pub mod transcription_manager;

pub use state::{AppState, SettingsSection, SidebarTab, View};

use dioxus::prelude::*;

use app::{
    load_consent, load_lang, use_app_contexts, use_history_tracker,
    use_picker_reset_on_view, use_sync_watcher, use_transcription_watcher,
    AppContexts, AppRouter,
};
use consent::ConsentScreen;
use restore_lock::RestoreLockScreen;

#[component]
pub fn App() -> Element {
    let AppContexts {
        db,
        engine,
        recorder: _recorder,
        manager,
        restore_locked,
        index_rebuilding,
    } = use_app_contexts();

    let app = use_context_provider(|| {
        let d = db();
        AppState::new(load_consent(&d), load_lang(&d))
    });

    use_transcription_watcher(manager.clone(), db, engine, app);
    use_sync_watcher(engine, app, restore_locked, index_rebuilding);
    use_picker_reset_on_view(app);
    use_history_tracker(app);

    #[cfg(target_os = "macos")]
    keyboard::use_macos_shortcuts(app);
    keyboard::use_keyboard_inset();

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        if restore_locked() {
            RestoreLockScreen {}
        } else if (app.ai_consent)() != Some(true) {
            ConsentScreen {}
        } else {
            AppRouter { index_rebuilding }
        }
    }
}
