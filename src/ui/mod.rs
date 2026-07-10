mod app;
mod chat;
mod clipboard;
pub mod hooks;
pub mod icons;
mod keyboard;
pub(crate) mod kit;
mod notes;
mod recording;
mod settings;
mod sidebar;
mod state;
mod sync;
mod thread;

pub use state::{AppState, RowMenu, SettingsSection, SidebarTab, View};

use dioxus::prelude::*;

use app::consent::ConsentScreen;
use app::restore_lock::RestoreLockScreen;
use app::{
    load_consent, load_lang, use_app_contexts, use_history_tracker,
    use_picker_reset_on_view, use_record_deeplink_watcher,
    use_share_inbox_watcher, use_sync_watcher, use_transcription_watcher,
    AppContexts, AppRouter,
};

#[component]
pub fn App() -> Element {
    let AppContexts {
        db,
        engine,
        recorder,
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
    use_record_deeplink_watcher(app, recorder);
    use_share_inbox_watcher(app, db);

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
