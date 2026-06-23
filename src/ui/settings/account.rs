use crate::application::i18n::t;
use crate::infrastructure::backend::{Account, BackendClient};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconArrowUpRight, IconCheck, IconCopy, IconTrash};
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn AccountSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    let this_device =
        use_signal(|| BackendClient::device_pubkey(&db()).unwrap_or_default());
    let mut account = use_signal(|| None::<Account>);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut busy = use_signal(|| false);
    let mut confirm_leave = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _trigger = reload();
        spawn(async move {
            let database = db();
            match BackendClient::from_db(&database) {
                None => account.set(None),
                Some(client) => match client.account(&database).await {
                    Ok(a) => {
                        account.set(Some(a));
                        status.set(None);
                    }
                    Err(e) => status.set(Some(e.to_string())),
                },
            }
        });
    });

    let has_backend = BackendClient::from_db(&db()).is_some();

    rsx! {
        div { class: "space-y-7 pb-20",
            p { class: "text-xs text-stone-500 leading-relaxed px-0.5",
                {t(&lang, "account-description")}
            }

            if !has_backend {
                div { class: "rounded-xl border border-ios-orange/40 bg-ios-orange/5 p-3",
                    p { class: "text-xs text-stone-700", {t(&lang, "account-no-backend")} }
                }
            }

            if let Some(err) = status() {
                div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                    p { class: "text-xs text-stone-600 break-words", "{err}" }
                }
            }

            if let Some(acc) = account() {
                // Identity card: account id + status badge, then the device-capacity meter.
                div { class: "bg-warm-white rounded-xl border border-stone-200 p-5",
                    div { class: "flex items-start justify-between gap-3 mb-4",
                        div { class: "min-w-0 flex-1",
                            label { class: "block text-xs font-medium text-stone-400 mb-1.5",
                                {t(&lang, "account-id-label")}
                            }
                            div { class: "flex items-center gap-2",
                                code { class: "font-mono text-[13px] text-stone-700 break-all",
                                    "{acc.account_id}"
                                }
                                button {
                                    class: "shrink-0 p-1.5 rounded-md text-stone-400 hover:bg-stone-100 active:bg-stone-100 transition-colors",
                                    onclick: {
                                        let id = acc.account_id.clone();
                                        move |_| crate::ui::clipboard::copy_text(&id)
                                    },
                                    IconCopy { size: 16 }
                                }
                            }
                        }
                        if acc.premium {
                            span { class: "shrink-0 inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-ios-orange/10 text-ios-orange-dark text-[11px] font-semibold border border-ios-orange/20",
                                IconCheck { size: 12 }
                                {t(&lang, "account-premium")}
                            }
                        } else {
                            span { class: "shrink-0 inline-flex items-center px-2.5 py-1 rounded-full bg-stone-100 text-stone-500 text-[11px] font-medium border border-stone-200",
                                {t(&lang, "account-free")}
                            }
                        }
                    }

                    div { class: "h-px bg-stone-100 -mx-5 mb-4" }

                    div {
                        div { class: "flex justify-between items-center mb-2.5",
                            span { class: "text-[13px] font-medium text-stone-700",
                                {t(&lang, "account-capacity-label")}
                            }
                            span { class: "font-mono text-[13px] text-stone-400 tabular-nums",
                                {format!("{} / {}", acc.devices.len(), acc.device_cap)}
                            }
                        }
                        div { class: "flex gap-1.5 h-1.5",
                            for i in 0..acc.device_cap {
                                div {
                                    key: "{i}",
                                    class: if (i as usize) < acc.devices.len() {
                                        "flex-1 rounded-full bg-ios-orange"
                                    } else {
                                        "flex-1 rounded-full bg-stone-200"
                                    },
                                }
                            }
                        }
                    }
                }

                div {
                    h3 { class: "text-[11px] font-medium text-stone-400 uppercase tracking-wide px-1 mb-2.5",
                        {t(&lang, "account-members-label")}
                    }
                    div { class: "bg-warm-white rounded-xl border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                        for d in acc.devices.clone() {
                            div {
                                key: "{d.device_id}",
                                class: "flex items-center gap-3.5 p-4",
                                div { class: "w-9 h-9 rounded-full bg-ios-orange/10 text-ios-orange-dark flex items-center justify-center font-semibold text-[13px] shrink-0",
                                    {monogram(&d.device_id)}
                                }
                                div { class: "flex-1 min-w-0",
                                    code { class: "block font-mono text-[13px] text-stone-700 truncate",
                                        {short_id(&d.device_id)}
                                    }
                                    if d.device_id == this_device() {
                                        span { class: "inline-block mt-1 text-[10px] font-medium text-ios-orange-dark bg-ios-orange/10 px-1.5 py-0.5 rounded",
                                            {t(&lang, "account-this-device")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if has_backend {
                p { class: "text-xs text-stone-400", {t(&lang, "account-loading")} }
            }

            // Zone sensible: leave (drop to solo/free, notes kept) + delete (hard local wipe + leave).
            div { class: "pt-7 border-t border-stone-200",
                h3 { class: "text-[11px] font-semibold text-stone-400 uppercase tracking-wide mb-4 px-1",
                    {t(&lang, "account-danger-title")}
                }
                div { class: "space-y-5",
                    div {
                        if confirm_leave() {
                            div { class: "flex items-center gap-2",
                                button {
                                    class: crate::ui::kit::CONFIRM_BTN_GHOST,
                                    onclick: move |_| confirm_leave.set(false),
                                    {t(&lang, "account-cancel")}
                                }
                                button {
                                    class: crate::ui::kit::CONFIRM_BTN_PRIMARY,
                                    disabled: busy(),
                                    onclick: move |_| {
                                        if busy() { return; }
                                        busy.set(true);
                                        confirm_leave.set(false);
                                        status.set(None);
                                        spawn(async move {
                                            let database = db();
                                            if let Some(client) = BackendClient::from_db(&database) {
                                                if let Err(e) = client.leave(&database).await {
                                                    status.set(Some(e.to_string()));
                                                }
                                            }
                                            busy.set(false);
                                            reload.set(reload() + 1);
                                        });
                                    },
                                    {t(&lang, "account-leave-confirm")}
                                }
                            }
                        } else {
                            button {
                                class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-warm-white border border-stone-200 text-stone-800 text-sm font-medium hover:bg-stone-50 active:bg-stone-100 transition-colors disabled:opacity-45",
                                disabled: busy() || !has_backend,
                                onclick: move |_| confirm_leave.set(true),
                                span { class: "text-stone-400", IconArrowUpRight { size: 16 } }
                                {t(&lang, "account-leave")}
                            }
                            p { class: "text-xs text-stone-500 mt-2 px-1 leading-relaxed",
                                {t(&lang, "account-leave-hint")}
                            }
                        }
                    }

                    div {
                        if confirm_delete() {
                            div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                                p { class: "text-xs text-stone-700 mb-2.5", {t(&lang, "account-delete-warn")} }
                                div { class: "flex items-center gap-2",
                                    button {
                                        class: crate::ui::kit::CONFIRM_BTN_GHOST,
                                        onclick: move |_| confirm_delete.set(false),
                                        {t(&lang, "account-cancel")}
                                    }
                                    button {
                                        class: crate::ui::kit::CONFIRM_BTN_DANGER,
                                        disabled: busy(),
                                        onclick: move |_| {
                                            if busy() { return; }
                                            busy.set(true);
                                            confirm_delete.set(false);
                                            status.set(None);
                                            spawn(async move {
                                                let database = db();
                                                delete_my_data(&database).await;
                                                app.notes_version.set((app.notes_version)() + 1);
                                                app.folders_version.set((app.folders_version)() + 1);
                                                busy.set(false);
                                                app.view.set(View::NotesList);
                                            });
                                        },
                                        {t(&lang, "account-delete-confirm")}
                                    }
                                }
                            }
                        } else {
                            button {
                                class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-ios-red-50 border border-ios-red/15 text-ios-red-dark text-sm font-medium hover:bg-ios-red/10 active:bg-ios-red/10 transition-colors disabled:opacity-45",
                                disabled: busy(),
                                onclick: move |_| confirm_delete.set(true),
                                IconTrash { size: 16 }
                                {t(&lang, "account-delete")}
                            }
                            p { class: "text-xs text-stone-500 mt-2 px-1 leading-relaxed",
                                {t(&lang, "account-delete-hint")}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn short_id(id: &str) -> String {
    if id.chars().count() > 16 {
        let head: String = id.chars().take(16).collect();
        format!("{head}...")
    } else {
        id.to_string()
    }
}

// Two-letter monogram for the device avatar.
fn monogram(id: &str) -> String {
    id.chars().take(2).collect()
}

// Hard local wipe + leave the cluster (RFC 0009 Q1.6). Local content first (so the data is gone even
// if the network call fails), then the backend leave (entitlement purge). Both best-effort.
async fn delete_my_data(db: &Database) {
    match db.wipe_local_content() {
        Ok(audio_paths) => {
            let audio_dir = crate::infrastructure::audio::output_dir();
            let dir = std::path::Path::new(&audio_dir);
            for p in &audio_paths {
                let _ = std::fs::remove_file(dir.join(p));
            }
            let _ = std::fs::remove_dir_all(
                crate::infrastructure::vectordb::vectordb_path(),
            );
        }
        Err(e) => eprintln!("[account] local wipe failed: {e}"),
    }
    if let Some(client) = BackendClient::from_db(db) {
        if let Err(e) = client.leave(db).await {
            eprintln!("[account] leave failed: {e}");
        }
    }
}
