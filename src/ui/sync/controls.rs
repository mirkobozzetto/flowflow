use crate::db::Database;
use crate::services::i18n::t;
use crate::services::sync::engine::{SyncActivity, SyncEngine};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// "Sync now" + the visible indicator (RFC T20 accept): current state, last
// result with counts, last error kept on screen - a stalled sync must be
// seen, never guessed.
#[component]
pub fn SyncControls() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<SyncEngine>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut activity = use_signal(|| engine.peek().activity());
    let mut has_peers = use_signal(|| false);

    // Activity polls the engine's in-memory mutex (cheap); the peers check
    // hits SQLite, whose single connection a running sync batch can hold for
    // tens of ms - poll it 4x less so the screen never stutters mid-sync.
    use_future(move || async move {
        let mut tick = 0u32;
        loop {
            let a = engine.peek().activity();
            if *activity.peek() != a {
                activity.set(a);
            }
            if tick.is_multiple_of(4) {
                let peers =
                    !db.peek().list_peers().unwrap_or_default().is_empty();
                if *has_peers.peek() != peers {
                    has_peers.set(peers);
                }
            }
            tick = tick.wrapping_add(1);
            futures_timer::Delay::new(std::time::Duration::from_millis(500))
                .await;
        }
    });

    let syncing = matches!(activity(), SyncActivity::Syncing);

    rsx! {
        div {
            h2 { class: "text-lg font-semibold text-stone-900 mb-1",
                {t(&lang, "sync-status-title")}
            }
            match activity() {
                SyncActivity::Idle => rsx! {
                    p { class: "text-xs text-stone-500 mb-3",
                        {t(&lang, "sync-status-idle")}
                    }
                },
                SyncActivity::Syncing => rsx! {
                    p { class: "text-xs text-stone-500 mb-3 animate-pulse",
                        {t(&lang, "sync-status-syncing")}
                    }
                },
                SyncActivity::Done { at, pushed, applied, conflicts, partial } => rsx! {
                    p { class: "text-xs text-ios-green mb-1",
                        {format!(
                            "{} {at} · ↑{pushed} ↓{applied}{}",
                            t(&lang, "sync-status-done"),
                            if conflicts > 0 {
                                format!(" · {conflicts} {}", t(&lang, "sync-status-conflicts"))
                            } else {
                                String::new()
                            },
                        )}
                    }
                    if let Some(failed) = partial {
                        p { class: "text-xs text-ios-red mb-3 break-words",
                            {format!("{} {failed}", t(&lang, "sync-status-partial"))}
                        }
                    } else {
                        p { class: "mb-3" }
                    }
                },
                SyncActivity::Error { at, message } => rsx! {
                    p { class: "text-xs text-ios-red mb-3 break-words",
                        {format!(
                            "{} {at} · {message}",
                            t(&lang, "sync-status-error"),
                        )}
                    }
                },
            }
            if has_peers() {
                button {
                    class: crate::ui::kit::BTN_PRIMARY,
                    disabled: syncing,
                    onclick: move |_| {
                        engine.peek().sync_now();
                    },
                    {t(&lang, "sync-now")}
                }
            } else {
                p { class: "text-xs text-stone-400 text-center",
                    {t(&lang, "sync-no-peers-hint")}
                }
            }
        }
    }
}
