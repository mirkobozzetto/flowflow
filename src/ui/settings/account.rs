use crate::db::Database;
use crate::services::backend::{Account, BackendClient};
use crate::services::i18n::t;
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
        div { class: "space-y-6 pb-20",
            p { class: "text-xs text-stone-500 leading-relaxed",
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
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1",
                        {t(&lang, "account-id-label")}
                    }
                    div { class: "flex items-center gap-2",
                        code {
                            class: "flex-1 text-[11px] font-mono break-all bg-warm-white border border-stone-200 rounded-lg px-3 py-2 text-stone-700",
                            "{acc.account_id}"
                        }
                        button {
                            class: crate::ui::kit::PILL_GHOST,
                            onclick: {
                                let id = acc.account_id.clone();
                                move |_| crate::ui::clipboard::copy_text(&id)
                            },
                            {t(&lang, "connections-copy")}
                        }
                    }
                }

                div { class: "flex items-center gap-2",
                    if acc.premium {
                        span { class: "inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-ios-green/10 text-ios-green text-xs font-medium border border-ios-green/20",
                            {t(&lang, "account-premium")}
                        }
                    } else {
                        span { class: "inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-stone-100 text-stone-500 text-xs font-medium border border-stone-200",
                            {t(&lang, "account-free")}
                        }
                    }
                    span { class: "text-xs text-stone-400",
                        {format!("{} / {}", acc.devices.len(), acc.device_cap)}
                    }
                }

                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-2",
                        {t(&lang, "account-members-label")}
                    }
                    div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                        for d in acc.devices.clone() {
                            div {
                                key: "{d.device_id}",
                                class: "px-4 py-3 flex items-center gap-2",
                                code { class: "flex-1 text-[11px] font-mono break-all text-stone-600",
                                    {short_id(&d.device_id)}
                                }
                                if d.device_id == this_device() {
                                    span { class: "shrink-0 text-[10px] font-medium text-ios-orange-dark px-2 py-0.5 rounded-full bg-ios-orange/10",
                                        {t(&lang, "account-this-device")}
                                    }
                                }
                            }
                        }
                    }
                }
            } else if has_backend {
                p { class: "text-xs text-stone-400", {t(&lang, "account-loading")} }
            }

            // Leave: drop to a fresh solo/free account, local notes kept.
            div { class: "pt-2",
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
                        class: crate::ui::kit::PILL_GHOST,
                        disabled: busy() || !has_backend,
                        onclick: move |_| confirm_leave.set(true),
                        {t(&lang, "account-leave")}
                    }
                    p { class: "text-[11px] text-stone-400 mt-1.5 leading-relaxed",
                        {t(&lang, "account-leave-hint")}
                    }
                }
            }

            // Delete my data: leave the cluster + hard-wipe local content. Destructive, double confirm.
            div { class: "pt-2 border-t border-stone-100",
                if confirm_delete() {
                    div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3 mt-3",
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
                        class: format!("{} mt-3", crate::ui::kit::PILL_GHOST),
                        disabled: busy(),
                        onclick: move |_| confirm_delete.set(true),
                        span { class: "text-ios-red-dark", {t(&lang, "account-delete")} }
                    }
                    p { class: "text-[11px] text-stone-400 mt-1.5 leading-relaxed",
                        {t(&lang, "account-delete-hint")}
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

// Hard local wipe + leave the cluster (RFC 0009 Q1.6). Local content first (so the data is gone even
// if the network call fails), then the backend leave (entitlement purge). Both best-effort.
async fn delete_my_data(db: &Database) {
    match db.wipe_local_content() {
        Ok(audio_paths) => {
            let audio_dir = crate::services::audio::output_dir();
            let dir = std::path::Path::new(&audio_dir);
            for p in &audio_paths {
                let _ = std::fs::remove_file(dir.join(p));
            }
            let _ = std::fs::remove_dir_all(
                crate::services::vectordb::vectordb_path(),
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
