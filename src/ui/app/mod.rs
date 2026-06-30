mod animations;
mod boot;
mod contexts;
mod router;
mod watchers;

pub(crate) mod consent;
pub(crate) mod fab;
// Back/forward history nav is wired only through the macOS keyboard shortcuts.
#[cfg(target_os = "macos")]
pub(crate) mod nav;
pub(crate) mod restore_lock;
pub(crate) mod right_nav;
pub(crate) mod top_bar;

pub use boot::{load_consent, load_lang};
pub use contexts::{use_app_contexts, AppContexts};
pub use router::AppRouter;
pub use watchers::{
    use_history_tracker, use_picker_reset_on_view, use_sync_watcher,
    use_transcription_watcher,
};
