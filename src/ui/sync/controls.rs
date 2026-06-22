use crate::db::Database;
use crate::services::i18n::t;
use crate::services::sync::engine::{SyncActivity, SyncEngine};
use crate::ui::icons::IconCheck;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const ICON_REFRESH: &str = "<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.3'/></svg>";
const ICON_REFRESH_SPIN: &str = "<svg class='animate-spin' width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.3'/></svg>";
const ICON_WARNING: &str = "<svg width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='10'/><line x1='12' y1='8' x2='12' y2='12'/><line x1='12' y1='16' x2='12.01' y2='16'/></svg>";

const CHIP: &str =
    "w-8 h-8 rounded-full flex items-center justify-center shrink-0";
const SYNC_NOW_BTN: &str = "w-full min-h-[44px] rounded-xl bg-ios-orange/10 text-ios-orange-dark text-sm font-medium active:bg-ios-orange/20 transition-colors";

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
        match activity() {
            SyncActivity::Idle => rsx! {
                div { class: "bg-white border border-stone-200 rounded-xl p-3 space-y-3",
                    div { class: "flex items-center gap-3",
                        div { class: "{CHIP} bg-stone-100 text-stone-500", dangerous_inner_html: ICON_REFRESH }
                        div { class: "flex flex-col min-w-0",
                            span { class: "text-sm font-medium text-stone-900", {t(&lang, "sync-status-title")} }
                            span { class: "text-xs text-stone-500 truncate", {t(&lang, "sync-status-idle")} }
                        }
                    }
                    if has_peers() {
                        button {
                            class: SYNC_NOW_BTN,
                            onclick: move |_| { engine.peek().sync_now(); },
                            {t(&lang, "sync-now")}
                        }
                    } else {
                        p { class: "text-xs text-stone-400 text-center", {t(&lang, "sync-no-peers-hint")} }
                    }
                }
            },
            SyncActivity::Syncing => rsx! {
                div { class: "bg-white border border-stone-200 rounded-xl p-3 flex items-center gap-3",
                    div { class: "{CHIP} bg-ios-orange/10 text-ios-orange", dangerous_inner_html: ICON_REFRESH_SPIN }
                    span { class: "text-sm font-medium text-stone-900", {t(&lang, "sync-status-syncing")} }
                }
            },
            SyncActivity::Done { at, pushed, applied, conflicts, partial } => rsx! {
                div { class: "bg-white border border-stone-200 rounded-xl p-3 space-y-2",
                    div { class: "flex items-center gap-3",
                        div { class: "{CHIP} bg-ios-orange/10 text-ios-orange-dark", IconCheck { size: 18 } }
                        div { class: "flex flex-col min-w-0",
                            span { class: "text-sm font-medium text-stone-900", {t(&lang, "sync-status-done")} }
                            span { class: "text-xs text-stone-500 truncate",
                                {format!(
                                    "{at} · ↑{pushed} ↓{applied}{}",
                                    if conflicts > 0 {
                                        format!(" · {conflicts} {}", t(&lang, "sync-status-conflicts"))
                                    } else {
                                        String::new()
                                    },
                                )}
                            }
                        }
                    }
                    if let Some(failed) = partial {
                        p { class: "text-xs text-ios-red break-words pl-11",
                            {format!("{} {failed}", t(&lang, "sync-status-partial"))}
                        }
                    }
                    if has_peers() {
                        button {
                            class: SYNC_NOW_BTN,
                            onclick: move |_| { engine.peek().sync_now(); },
                            {t(&lang, "sync-now")}
                        }
                    }
                }
            },
            SyncActivity::Error { at, message } => rsx! {
                div { class: "bg-white border border-ios-red/20 rounded-xl p-3 space-y-3 relative overflow-hidden",
                    div { class: "absolute top-0 left-0 w-1 h-full bg-ios-red/80" }
                    div { class: "flex items-start gap-3 pl-1",
                        div { class: "{CHIP} bg-ios-red/10 text-ios-red mt-0.5", dangerous_inner_html: ICON_WARNING }
                        div { class: "flex flex-col gap-1 min-w-0",
                            span { class: "text-sm font-medium text-stone-900", {t(&lang, "sync-failed")} }
                            span { class: "text-xs text-stone-500 leading-relaxed break-words",
                                {format!("{} {at} · {message}", t(&lang, "sync-status-error"))}
                            }
                        }
                    }
                    if has_peers() {
                        button {
                            class: "w-full min-h-[44px] rounded-xl bg-white border border-stone-200 text-stone-700 text-sm font-medium active:bg-stone-50 transition-colors",
                            disabled: syncing,
                            onclick: move |_| { engine.peek().sync_now(); },
                            {t(&lang, "sync-now")}
                        }
                    }
                }
            },
        }
    }
}
