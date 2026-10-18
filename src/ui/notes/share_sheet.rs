use super::share_section::{expiry_label, share_error_key};
use crate::application::i18n::{t, t_args};
use crate::application::sharing;
use crate::domain::share::share_link;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::{device_log, haptic};
use crate::ui::hooks::swipe::use_sheet_dismiss;
use crate::ui::icons::{IconCheck, IconLink, IconLockSimple, IconX};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;
use std::time::Duration;

const SHEET_ID: &str = "share-sheet";
const BACKDROP_ID: &str = "share-backdrop";
/// The grabber and the title: a drag starting there pulls the sheet down.
const GRAB_PX: f64 = 56.0;
const DISMISS_AT: f64 = 0.3;
/// Matches `bottom-sheet-out` in tailwind.css.
const CLOSE_MS: u64 = 300;

const OPTION: &str = "pressable w-full flex items-center gap-3 min-h-[52px] px-3.5 rounded-2xl bg-stone-900/[0.045] text-[15px] text-stone-900 text-left active:bg-stone-900/[0.09] disabled:opacity-60";
const DOT_ON: &str = "w-[22px] h-[22px] shrink-0 rounded-full border-[1.5px] border-ios-orange bg-ios-orange flex items-center justify-center text-white";
const DOT_OFF: &str =
    "w-[22px] h-[22px] shrink-0 rounded-full border-[1.5px] border-stone-300";

/// A note's sharing: the « Partagée » chip under its date while a link
/// lives, and the sheet that the chip or « … › Partager cette note » opens.
#[component]
pub fn NoteShare(local_note_id: Signal<String>) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut open = use_signal(|| false);
    let version = use_signal(|| 0u32);

    use_effect(move || {
        let id = local_note_id();
        if id.is_empty() || (app.share_request)().as_deref() != Some(&id) {
            return;
        }
        app.share_request.set(None);
        open.set(true);
    });

    let _ = version();
    let id = local_note_id();
    let shared = !id.is_empty() && db().get_share(&id).is_some();

    rsx! {
        if shared {
            button {
                class: "pressable mt-2 ml-1 inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                onclick: move |_| open.set(true),
                IconLink { size: 12 }
                {t(&lang, "share-chip")}
            }
        }
        if open() {
            ShareSheet { local_note_id, version, on_close: move |_| open.set(false) }
        }
    }
}

/// Choosing the access publishes or cuts the link without copying anything;
/// « Copier le lien » copies then closes, « Privée » cuts then closes.
#[component]
fn ShareSheet(
    local_note_id: Signal<String>,
    version: Signal<u32>,
    on_close: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut version = version;
    let mut closing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    use_sheet_dismiss(SHEET_ID, BACKDROP_ID, GRAB_PX, DISMISS_AT, move || {
        haptic("soft");
        on_close.call(());
    });
    use_hook(|| haptic("soft"));

    let mut close = move || {
        if closing() {
            return;
        }
        closing.set(true);
        spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(CLOSE_MS)).await;
            on_close.call(());
        });
    };

    let _ = version();
    let share = db().get_share(&local_note_id());
    let public = share.is_some();
    let publishing = busy() && !public;

    let make_public = move |_| {
        if public || busy() {
            return;
        }
        haptic("light");
        busy.set(true);
        error.set(None);
        spawn(async move {
            let id = local_note_id();
            match sharing::publish_note(&db(), &id).await {
                Ok(_) => device_log(&format!("[share] {id}: public")),
                Err(e) => {
                    device_log(&format!("[share] {id}: publish failed: {e}"));
                    let lang = (app.current_lang)();
                    error.set(Some(t(&lang, share_error_key(&e))));
                }
            }
            busy.set(false);
            version += 1;
        });
    };

    let make_private = move |_| {
        if busy() {
            return;
        }
        haptic("light");
        if !public {
            close();
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            let mut close = close;
            let id = local_note_id();
            match sharing::revoke(&db(), &id).await {
                Ok(()) => {
                    device_log(&format!("[share] {id}: private"));
                    close();
                }
                Err(e) => {
                    device_log(&format!("[share] {id}: revoke failed: {e}"));
                    let lang = (app.current_lang)();
                    error.set(Some(t(&lang, share_error_key(&e))));
                }
            }
            busy.set(false);
            version += 1;
        });
    };

    let copy_and_close = {
        let code = share.as_ref().map(|s| s.code.clone());
        move |_| {
            let Some(code) = &code else {
                return;
            };
            crate::ui::clipboard::copy_text(&share_link(code));
            device_log(&format!("[share] {}: link copied", local_note_id()));
            haptic("light");
            close();
        }
    };

    rsx! {
        div {
            id: BACKDROP_ID,
            class: "bottom-sheet-backdrop fixed inset-0 z-40 bg-stone-900/20",
            "data-closing": closing().then_some("true"),
            onclick: move |_| {
                haptic("soft");
                close();
            },
        }
        div {
            id: SHEET_ID,
            class: "bottom-sheet glass-panel fixed left-2 right-2 z-50 rounded-[38px] px-5 pt-2 pb-[22px]",
            style: "bottom: calc(env(safe-area-inset-bottom) + 8px);",
            "data-closing": closing().then_some("true"),
            role: "dialog",
            div { class: "w-9 h-[5px] mx-auto mb-2.5 rounded-full bg-stone-300" }
            div { class: "flex items-center gap-2.5",
                span { class: "text-[15px] font-semibold text-stone-900",
                    {t(&lang, "share-publish-note")}
                }
                button {
                    class: "pressable ml-auto glass-disc relative w-9 h-9 -mr-1 flex items-center justify-center rounded-full text-stone-500",
                    aria_label: t(&lang, "reading-close"),
                    onclick: move |_| {
                        haptic("soft");
                        close();
                    },
                    IconX { size: 16 }
                }
            }
            p { class: "mt-3 mb-2 text-xs font-medium text-stone-400 uppercase tracking-[0.08em]",
                {t(&lang, "share-access")}
            }
            button {
                class: OPTION,
                aria_pressed: public,
                disabled: busy() && public,
                onclick: make_public,
                span { class: if public { DOT_ON } else { DOT_OFF },
                    if public {
                        IconCheck { size: 13 }
                    }
                }
                span { class: "flex-1", {t(&lang, "share-anyone")} }
                if publishing {
                    span { class: "text-xs text-stone-400 animate-pulse",
                        {t(&lang, "share-publishing")}
                    }
                } else {
                    span { class: "text-stone-400", IconLink { size: 18 } }
                }
            }
            button {
                class: "{OPTION} mt-2",
                aria_pressed: !public && !publishing,
                disabled: publishing,
                onclick: make_private,
                span { class: if !public && !publishing { DOT_ON } else { DOT_OFF },
                    if !public && !publishing {
                        IconCheck { size: 13 }
                    }
                }
                span { class: "flex-1", {t(&lang, "share-private")} }
                span { class: "text-stone-400", IconLockSimple { size: 18 } }
            }
            p { class: "mt-3 px-1 min-h-[34px] text-xs leading-relaxed text-stone-500",
                if let Some(msg) = error() {
                    span { class: "text-ios-red", "{msg}" }
                } else if let Some(s) = &share {
                    {t_args(&lang, "share-active-hint", &[("date", &expiry_label(&s.expires_at))])}
                }
            }
            button {
                class: "pressable w-full h-[50px] mt-2 rounded-full bg-ios-orange text-white text-base font-semibold transition-opacity disabled:opacity-40",
                disabled: !public || busy(),
                onclick: copy_and_close,
                {t(&lang, "share-copy-link")}
            }
        }
    }
}
