use crate::application::device_naming::ensure_device_name;
use crate::application::i18n::{t, t_args};
use crate::infrastructure::backend::{Account, BackendClient};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{
    IconArrowUpRight, IconArrowsClockwise, IconCheck, IconCopy,
    IconDeviceLaptop, IconDevicePhone, IconShieldCheck, IconTrash,
};
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

const ACCOUNT_SITE_URL: &str = "https://account.flowflow.be";

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
    let mut confirm_remove: Signal<Option<String>> = use_signal(|| None);
    // The heal notice shows ONCE: consumed at mount, gone on the next visit.
    let heal_event: Signal<Option<String>> = use_signal(|| {
        let database = db();
        let v = database
            .get_setting(crate::application::account_heal::HEAL_EVENT_KEY)
            .filter(|s| !s.is_empty());
        if v.is_some() {
            let _ = database.set_setting(
                crate::application::account_heal::HEAL_EVENT_KEY,
                "",
            );
        }
        v
    });
    let mut link_code: Signal<Option<(String, String)>> = use_signal(|| None);
    let mut id_copied = use_signal(|| false);
    let mut code_copied = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _trigger = reload();
        spawn(async move {
            let database = db();
            match BackendClient::from_db(&database) {
                None => account.set(None),
                Some(client) => match client.account(&database).await {
                    Ok(a) => {
                        crate::application::account_heal::cache_account(
                            &database, &a,
                        );
                        account.set(Some(a));
                        status.set(None);
                    }
                    Err(e) => status.set(Some(e.to_string())),
                },
            }
        });
    });

    let has_backend = BackendClient::from_db(&db()).is_some();
    let my_device_name = ensure_device_name(&db());
    let sync_peers = db().list_peers().unwrap_or_default();
    let has_sync_peer = !sync_peers.is_empty();
    // Until the backend carries names, the Mac already knows its sync peer's
    // name: when the cluster has exactly one other device and exactly one
    // named sync peer, that name labels the row. Backend name wins once set.
    let peer_name_fallback: Option<String> = account().as_ref().and_then(|a| {
        let others = a
            .devices
            .iter()
            .filter(|d| d.device_id != this_device())
            .count();
        let named: Vec<&String> =
            sync_peers.iter().filter_map(|p| p.name.as_ref()).collect();
        if others == 1 && named.len() == 1 {
            Some(named[0].clone())
        } else {
            None
        }
    });

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
                // Plan hero: the status first; the upgrade path goes through the web site.
                div { class: "bg-warm-white rounded-xl border border-stone-200 p-5",
                    label { class: "block text-xs font-medium text-stone-400 mb-1.5",
                        {t(&lang, "account-plan-title")}
                    }
                    div { class: "flex items-center gap-2 mb-1.5",
                        if acc.premium {
                            span { class: "text-ios-orange-dark", IconShieldCheck { size: 20 } }
                            span { class: "text-xl font-semibold text-stone-800",
                                {t(&lang, "account-premium")}
                            }
                        } else {
                            span { class: "text-xl font-semibold text-stone-800",
                                {t(&lang, "account-free")}
                            }
                        }
                    }
                    p { class: "text-xs text-stone-500 leading-relaxed mb-3",
                        {t(&lang, "account-plan-hint")}
                    }
                    if !acc.premium {
                        button {
                            class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-ios-orange text-white text-sm font-medium active:opacity-80 transition-opacity",
                            onclick: move |_| {
                                crate::infrastructure::platform::open_url(ACCOUNT_SITE_URL)
                            },
                            {t(&lang, "account-plan-upgrade")}
                            IconArrowUpRight { size: 16 }
                        }
                    }
                }

                if let Some(err) = db()
                    .get_setting(crate::application::account_heal::HEAL_ERROR_KEY)
                    .filter(|v| !v.is_empty())
                {
                    div { class: "rounded-xl border border-ios-orange/40 bg-ios-orange/5 p-3",
                        p { class: "text-xs text-stone-700 break-words", "{err}" }
                    }
                }

                div {
                    div { class: "flex items-baseline px-1 mb-2.5",
                        h3 { class: "text-[11px] font-medium text-stone-400 uppercase tracking-wide",
                            {t(&lang, "account-members-label")}
                        }
                        span { class: "ml-auto text-xs text-stone-400",
                            {t_args(&lang, "account-count", &[
                                ("n", &acc.devices.len().to_string()),
                                ("cap", &acc.device_cap.to_string()),
                            ])}
                        }
                    }
                    if acc.premium {
                        if let Some(name) = heal_event() {
                            p { class: "flex items-center gap-1.5 text-xs text-stone-400 px-1 mb-2.5",
                                span { class: "text-ios-orange-dark", IconShieldCheck { size: 12 } }
                                {t_args(&lang, "account-heal-event", &[("name", &name)])}
                            }
                        }
                    }
                    div { class: "bg-warm-white rounded-xl border border-stone-200 overflow-hidden",
                        div { class: "divide-y divide-stone-100",
                            for d in acc.devices.clone() {
                                div {
                                    key: "{d.device_id}",
                                    class: "flex items-center gap-4 px-4 py-3.5 min-h-[64px]",
                                    span { class: "text-stone-400 shrink-0",
                                        if d.device_id == this_device() {
                                            if cfg!(target_os = "ios") {
                                                IconDevicePhone { size: 34 }
                                            } else {
                                                IconDeviceLaptop { size: 34 }
                                            }
                                        } else {
                                            // Platform of a sibling is unknown; with two
                                            // devices the other one is almost always the
                                            // other form factor. A glyph, never an identity.
                                            if cfg!(target_os = "ios") {
                                                IconDeviceLaptop { size: 34 }
                                            } else {
                                                IconDevicePhone { size: 34 }
                                            }
                                        }
                                    }
                                    div { class: "flex-1 min-w-0",
                                        div { class: "text-[15px] font-semibold tracking-tight text-stone-800 truncate",
                                            {device_label(&d, &this_device(), &my_device_name, peer_name_fallback.as_deref())}
                                        }
                                        div { class: "flex items-center gap-1.5 text-xs text-stone-400 mt-0.5",
                                            if d.device_id == this_device() {
                                                span { class: "text-ios-orange-dark font-medium",
                                                    {t(&lang, "account-this-device")}
                                                }
                                                span { "·" }
                                                span {
                                                    if cfg!(target_os = "ios") { "iPhone" } else { "Mac" }
                                                }
                                            } else {
                                                if acc.premium {
                                                    span { class: "inline-flex items-center gap-1 text-ios-orange-dark font-medium",
                                                        IconShieldCheck { size: 12 }
                                                        {t(&lang, "account-premium")}
                                                    }
                                                    span { "·" }
                                                }
                                                span {
                                                    {t_args(&lang, "account-last-seen", &[("time", &seen_label(&d.last_seen))])}
                                                }
                                            }
                                        }
                                    }
                                    if d.device_id != this_device() {
                                        if confirm_remove().as_deref() == Some(d.device_id.as_str()) {
                                            div { class: "flex items-center gap-2 shrink-0",
                                                button {
                                                    class: "h-8 px-3 rounded-lg bg-stone-100 text-stone-900 text-xs font-medium active:bg-stone-200 transition-colors",
                                                    onclick: move |_| confirm_remove.set(None),
                                                    {t(&lang, "account-cancel")}
                                                }
                                                button {
                                                    class: "h-8 px-3 rounded-lg bg-ios-red text-white text-xs font-medium active:opacity-80 transition-opacity disabled:opacity-45",
                                                    disabled: busy(),
                                                    onclick: {
                                                        let target = d.device_id.clone();
                                                        move |_| {
                                                            if busy() { return; }
                                                            busy.set(true);
                                                            status.set(None);
                                                            confirm_remove.set(None);
                                                            let target = target.clone();
                                                            spawn(async move {
                                                                let database = db();
                                                                if let Some(client) =
                                                                    BackendClient::from_db(&database)
                                                                {
                                                                    if let Err(e) = client
                                                                        .remove_device(&database, &target)
                                                                        .await
                                                                    {
                                                                        status.set(Some(e.to_string()));
                                                                    }
                                                                }
                                                                busy.set(false);
                                                                reload.set(reload() + 1);
                                                            });
                                                        }
                                                    },
                                                    {t(&lang, "account-remove-device")}
                                                }
                                            }
                                        } else {
                                            button {
                                                class: "shrink-0 px-2 py-1.5 text-xs font-medium text-stone-400 hover:text-ios-red-dark active:text-ios-red-dark transition-colors disabled:opacity-45",
                                                disabled: busy() || !has_backend,
                                                onclick: {
                                                    let target = d.device_id.clone();
                                                    move |_| confirm_remove.set(Some(target.clone()))
                                                },
                                                {t(&lang, "account-remove-device")}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex items-center gap-2 px-4 py-2.5 bg-stone-100/50 border-t border-stone-100",
                            span { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide shrink-0",
                                {t(&lang, "account-id-label")}
                            }
                            code { class: "flex-1 font-mono text-[11px] text-stone-400 truncate",
                                "{acc.account_id}"
                            }
                            button {
                                class: if id_copied() {
                                    "shrink-0 p-1.5 rounded-md text-ios-green transition-colors"
                                } else {
                                    "shrink-0 p-1.5 rounded-md text-stone-400 hover:bg-stone-100 active:bg-stone-100 transition-colors"
                                },
                                onclick: {
                                    let id = acc.account_id.clone();
                                    move |_| {
                                        if id_copied() { return; }
                                        crate::ui::clipboard::copy_text(&id);
                                        id_copied.set(true);
                                        spawn(async move {
                                            futures_timer::Delay::new(
                                                std::time::Duration::from_millis(1500),
                                            )
                                            .await;
                                            id_copied.set(false);
                                        });
                                    }
                                },
                                if id_copied() {
                                    IconCheck { size: 16 }
                                } else {
                                    IconCopy { size: 16 }
                                }
                            }
                        }
                    }
                    if !acc.premium && has_sync_peer {
                        div { class: "mt-3",
                            p { class: "text-xs text-stone-500 leading-relaxed mb-2.5 px-1",
                                {t(&lang, "account-join-hint")}
                            }
                            button {
                                class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-ios-orange/10 text-ios-orange-dark text-sm font-medium active:bg-ios-orange/25 transition-colors",
                                onclick: move |_| app.view.set(View::SyncPairing),
                                IconArrowsClockwise { size: 16 }
                                {t(&lang, "account-join")}
                            }
                        }
                    }
                }

                div {
                    h3 { class: "text-[11px] font-medium text-stone-400 uppercase tracking-wide px-1 mb-2.5",
                        {t(&lang, "account-link-title")}
                    }
                    div { class: "bg-warm-white rounded-xl border border-stone-200 p-5 space-y-3",
                        if let Some((code, exp)) = link_code() {
                            div {
                                label { class: "block text-xs font-medium text-stone-400 mb-1.5",
                                    {t(&lang, "account-link-code-label")}
                                }
                                div { class: "flex items-center gap-2",
                                    code { class: "flex-1 font-mono text-[13px] text-stone-700 break-all",
                                        "{code}"
                                    }
                                    button {
                                        class: if code_copied() {
                                            "shrink-0 p-1.5 rounded-md text-ios-green transition-colors"
                                        } else {
                                            "shrink-0 p-1.5 rounded-md text-stone-400 hover:bg-stone-100 active:bg-stone-100 transition-colors"
                                        },
                                        onclick: {
                                            let c = code.clone();
                                            move |_| {
                                                if code_copied() { return; }
                                                crate::ui::clipboard::copy_text(&c);
                                                code_copied.set(true);
                                                spawn(async move {
                                                    futures_timer::Delay::new(
                                                        std::time::Duration::from_millis(1500),
                                                    )
                                                    .await;
                                                    code_copied.set(false);
                                                });
                                            }
                                        },
                                        if code_copied() {
                                            IconCheck { size: 16 }
                                        } else {
                                            IconCopy { size: 16 }
                                        }
                                    }
                                }
                                p { class: "text-xs text-stone-500 mt-2",
                                    {t_args(&lang, "account-link-expires", &[("time", &link_expiry_label(&exp))])}
                                }
                            }
                        } else {
                            p { class: "text-xs text-stone-500 leading-relaxed",
                                {t(&lang, "account-link-hint")}
                            }
                        }
                        button {
                            class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-warm-white border border-stone-200 text-stone-800 text-sm font-medium hover:bg-stone-50 active:bg-stone-100 transition-colors disabled:opacity-45",
                            disabled: busy() || !has_backend,
                            onclick: move |_| {
                                if busy() { return; }
                                busy.set(true);
                                status.set(None);
                                spawn(async move {
                                    let database = db();
                                    if let Some(client) = BackendClient::from_db(&database) {
                                        match client.link_begin(&database).await {
                                            Ok(pair) => link_code.set(Some(pair)),
                                            Err(e) => status.set(Some(e.to_string())),
                                        }
                                    }
                                    busy.set(false);
                                });
                            },
                            span { class: "text-stone-400", IconArrowUpRight { size: 16 } }
                            {
                                if busy() {
                                    t(&lang, "account-link-generating")
                                } else if link_code().is_some() {
                                    t(&lang, "account-link-new")
                                } else {
                                    t(&lang, "account-link-button")
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

// Render the server's ISO expiry as a local wall-clock time; fall back to the raw string if it
// ever fails to parse, so the user always sees something.
fn link_expiry_label(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn short_id(id: &str) -> String {
    if id.chars().count() > 16 {
        let head: String = id.chars().take(16).collect();
        format!("{head}...")
    } else {
        id.to_string()
    }
}

// What a member row displays: my own local name for this device, the name the
// peer pushed to the backend, else the truncated id.
fn device_label(
    d: &crate::infrastructure::backend::MemberDevice,
    my_id: &str,
    my_name: &str,
    peer_fallback: Option<&str>,
) -> String {
    if d.device_id == my_id {
        return my_name.to_string();
    }
    d.name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .or_else(|| peer_fallback.map(str::to_string))
        .unwrap_or_else(|| short_id(&d.device_id))
}

// Local wall-clock label for a member's last_seen; raw prefix if it ever
// fails to parse.
fn seen_label(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%d/%m %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| iso.chars().take(10).collect())
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
