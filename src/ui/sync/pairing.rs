use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::deeplink;
use crate::infrastructure::sync::peers::{self, PairingHost, PairingStatus};
use crate::ui::clipboard::copy_text;
use crate::ui::icons::{IconCheck, IconCopy};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn SyncPairingView() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut host: Signal<Option<PairingHost>> = use_signal(|| None);
    let mut host_status: Signal<Option<PairingStatus>> = use_signal(|| None);
    let mut join_uri = use_signal(String::new);
    let mut join_status: Signal<Option<Result<String, String>>> =
        use_signal(|| None);
    let mut joining = use_signal(|| false);
    let mut peers_version = use_signal(|| 0u32);
    let mut scanned = use_signal(|| false);
    let mut uri_copied = use_signal(|| false);

    let peers_list = use_memo(move || {
        peers_version();
        db().list_peers().unwrap_or_default()
    });

    use_future(move || async move {
        loop {
            if let Some(uri) = deeplink::take() {
                join_uri.set(uri);
                scanned.set(true);
                join_status.set(None);
            }
            futures_timer::Delay::new(std::time::Duration::from_millis(400))
                .await;
        }
    });

    use_future(move || async move {
        loop {
            let current = host.peek().clone();
            if let Some(h) = current {
                let s = h.status();
                if host_status.peek().as_ref() != Some(&s) {
                    match &s {
                        PairingStatus::Paired { .. } => {
                            peers_version += 1;
                        }
                        PairingStatus::Failed(_) | PairingStatus::Cancelled => {
                            // Listener thread is gone: drop the dead QR so the
                            // start button reappears for a fresh attempt.
                            host.set(None);
                        }
                        PairingStatus::Waiting => {}
                    }
                    host_status.set(Some(s));
                }
            }
            futures_timer::Delay::new(std::time::Duration::from_millis(400))
                .await;
        }
    });

    use_drop(move || {
        if let Some(h) = host.peek().as_ref() {
            h.cancel();
        }
    });

    rsx! {
        div { class: "space-y-6 border-t border-stone-200 pt-4",

            // ---- Add a device ----
            div { class: "space-y-4",
                h2 { class: "text-lg font-semibold text-stone-900",
                    {t(&lang, "settings-pair-device")}
                }
                p { class: "text-xs text-stone-500 leading-relaxed",
                    {t(&lang, "sync-show-code-hint")}
                }

                if let Some(h) = host() {
                    {
                        let uri_for_copy = h.uri.clone();
                        rsx! {
                            div { class: "bg-white rounded-xl border border-stone-200 p-5 flex flex-col items-center",
                                div { class: "relative w-[200px] h-[200px] bg-white border border-stone-200 rounded-2xl p-3 mb-4",
                                    div { class: "absolute top-2 left-2 w-4 h-4 border-t-2 border-l-2 border-ios-orange rounded-tl" }
                                    div { class: "absolute top-2 right-2 w-4 h-4 border-t-2 border-r-2 border-ios-orange rounded-tr" }
                                    div { class: "absolute bottom-2 left-2 w-4 h-4 border-b-2 border-l-2 border-ios-orange rounded-bl" }
                                    div { class: "absolute bottom-2 right-2 w-4 h-4 border-b-2 border-r-2 border-ios-orange rounded-br" }
                                    div {
                                        class: "w-full h-full flex items-center justify-center [&_svg]:max-w-full [&_svg]:h-auto",
                                        dangerous_inner_html: "{h.qr_svg}",
                                    }
                                }
                                span { class: "text-sm font-medium text-stone-900 font-mono bg-stone-100 px-3 py-1 rounded-md",
                                    "{h.addr}:{h.port}"
                                }
                                div { class: "mt-3 w-full bg-warm-white rounded-lg pl-3 pr-1 py-1.5 flex items-center justify-between border border-stone-200",
                                    span {
                                        class: "text-[11px] text-stone-500 font-mono truncate mr-2",
                                        style: "user-select: all; -webkit-user-select: all;",
                                        "{h.uri}"
                                    }
                                    button {
                                        class: if uri_copied() {
                                            "text-ios-green shrink-0 w-8 h-8 flex items-center justify-center rounded-md transition-colors"
                                        } else {
                                            "text-ios-orange shrink-0 w-8 h-8 flex items-center justify-center rounded-md active:bg-ios-orange/10 transition-colors"
                                        },
                                        onclick: move |_| {
                                            if uri_copied() { return; }
                                            copy_text(&uri_for_copy);
                                            uri_copied.set(true);
                                            spawn(async move {
                                                futures_timer::Delay::new(
                                                    std::time::Duration::from_millis(1500),
                                                )
                                                .await;
                                                uri_copied.set(false);
                                            });
                                        },
                                        if uri_copied() {
                                            IconCheck { size: 14 }
                                        } else {
                                            IconCopy { size: 14 }
                                        }
                                    }
                                }
                                div { class: "mt-4 flex justify-center w-full",
                                    match host_status() {
                                        Some(PairingStatus::Paired { device_id }) => rsx! {
                                            div { class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-ios-orange/10 text-ios-orange-dark text-xs font-medium border border-ios-orange/20",
                                                IconCheck { size: 12 }
                                                {format!("{} {}", t(&lang, "sync-paired-with"), device_id)}
                                            }
                                        },
                                        Some(PairingStatus::Failed(e)) => rsx! {
                                            div { class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-ios-red/10 text-ios-red text-xs font-medium border border-ios-red/20",
                                                {format!("{}: {}", t(&lang, "sync-failed"), e)}
                                            }
                                        },
                                        Some(PairingStatus::Cancelled) => rsx! {
                                            span { class: "text-xs text-stone-400", {t(&lang, "sync-cancelled")} }
                                        },
                                        _ => rsx! {
                                            div { class: "inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-ios-orange/10 text-ios-orange-dark text-xs font-medium border border-ios-orange/20",
                                                span { class: "relative flex h-2 w-2",
                                                    span { class: "animate-ping absolute inline-flex h-full w-full rounded-full bg-ios-orange opacity-75" }
                                                    span { class: "relative inline-flex rounded-full h-2 w-2 bg-ios-orange" }
                                                }
                                                {t(&lang, "sync-waiting")}
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                } else {
                    button {
                        class: crate::ui::kit::BTN_PRIMARY,
                        onclick: move |_| {
                            if let Some(prev) = host.peek().as_ref() {
                                prev.cancel();
                                return;
                            }
                            match peers::start_pairing_host(db()) {
                                Ok(h) => {
                                    host_status.set(Some(PairingStatus::Waiting));
                                    host.set(Some(h));
                                }
                                Err(e) => {
                                    host_status
                                        .set(Some(PairingStatus::Failed(e.to_string())));
                                    host.set(None);
                                }
                            }
                        },
                        {t(&lang, "sync-show-code")}
                    }
                    if let Some(PairingStatus::Failed(e)) = host_status() {
                        p { class: "text-sm text-ios-red text-center mt-2",
                            {format!("{}: {}", t(&lang, "sync-failed"), e)}
                        }
                    }
                }

                div { class: "flex items-center justify-center py-1",
                    div { class: "h-px bg-stone-200 w-full" }
                    span { class: "px-4 text-[11px] font-semibold text-stone-400 uppercase tracking-widest",
                        {t(&lang, "sync-or")}
                    }
                    div { class: "h-px bg-stone-200 w-full" }
                }

                if scanned() {
                    div { class: "bg-ios-orange/10 border border-ios-orange/40 rounded-xl px-3 py-2.5",
                        p { class: "text-sm text-stone-900 font-medium",
                            {t(&lang, "sync-scanned-title")}
                        }
                        p { class: "text-xs text-stone-600 leading-relaxed mt-0.5",
                            {t(&lang, "sync-scanned-hint")}
                        }
                    }
                } else {
                    p { class: "text-xs text-stone-500 leading-relaxed",
                        {t(&lang, "sync-join-hint")}
                    }
                }
                textarea {
                    class: "w-full border border-stone-200 rounded-xl px-3 py-2.5 text-xs font-mono outline-none text-stone-900 bg-warm-white min-h-[72px] focus:border-ios-orange-dark focus:ring-[3px] focus:ring-ios-orange/20 transition-colors duration-150",
                    placeholder: "flowflow://pair#...",
                    value: "{join_uri}",
                    oninput: move |evt| join_uri.set(evt.value()),
                }
                button {
                    class: if joining() {
                        "w-full min-h-[44px] rounded-xl text-sm font-medium bg-stone-300 text-white"
                    } else {
                        "w-full min-h-[44px] rounded-xl text-sm font-medium bg-ios-orange text-white active:bg-ios-orange-dark transition-colors"
                    },
                    disabled: joining(),
                    onclick: move |_| {
                        let uri = join_uri().trim().to_string();
                        if uri.is_empty() || joining() {
                            return;
                        }
                        joining.set(true);
                        join_status.set(None);
                        let db_arc = db();
                        spawn(async move {
                            let res = tokio::task::spawn_blocking(move || {
                                peers::join_pairing(&db_arc, &uri)
                            })
                            .await;
                            let outcome = match res {
                                Ok(Ok(id)) => Ok(id),
                                Ok(Err(e)) => Err(e.to_string()),
                                Err(e) => Err(e.to_string()),
                            };
                            if outcome.is_ok() {
                                peers_version += 1;
                                scanned.set(false);
                            }
                            join_status.set(Some(outcome));
                            joining.set(false);
                        });
                    },
                    if joining() {
                        {t(&lang, "sync-connecting")}
                    } else if scanned() {
                        {t(&lang, "sync-confirm-pairing")}
                    } else {
                        {t(&lang, "sync-connect")}
                    }
                }
                match join_status() {
                    Some(Ok(id)) => rsx! {
                        div { class: "flex justify-center mt-1",
                            div { class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-ios-orange/10 text-ios-orange-dark text-xs font-medium border border-ios-orange/20",
                                IconCheck { size: 12 }
                                {format!("{} {}", t(&lang, "sync-paired-with"), id)}
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        p { class: "text-sm text-ios-red text-center mt-1",
                            {format!("{}: {}", t(&lang, "sync-failed"), e)}
                        }
                    },
                    None => rsx! {},
                }
            }

            // ---- Paired devices ----
            div { class: "border-t border-stone-200 pt-4",
                h2 { class: "text-lg font-semibold text-stone-900 mb-3",
                    {t(&lang, "sync-peers-title")}
                }
                if peers_list().is_empty() {
                    p { class: "text-xs text-stone-400 text-center py-2",
                        {t(&lang, "sync-no-peers")}
                    }
                } else {
                    div { class: "space-y-2",
                        for peer in peers_list() {
                            {
                                let peer_id = peer.device_id.clone();
                                let initial = peer
                                    .device_id
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_uppercase()
                                    .to_string();
                                rsx! {
                                    div {
                                        class: "flex items-center justify-between bg-white border border-stone-200 rounded-xl px-3 py-2.5",
                                        div { class: "flex items-center gap-3 min-w-0",
                                            div { class: "w-10 h-10 rounded-full bg-warm-white border border-stone-200 flex items-center justify-center shrink-0 text-ios-orange text-sm font-semibold",
                                                "{initial}"
                                            }
                                            div { class: "min-w-0",
                                                p { class: "text-sm font-medium text-stone-900 truncate", "{peer.device_id}" }
                                                if let Some(ref at) = peer.paired_at {
                                                    p { class: "text-xs text-stone-500 truncate", "{at}" }
                                                }
                                            }
                                        }
                                        button {
                                            class: "text-xs font-medium text-stone-500 px-3 min-h-[44px] rounded-lg active:text-ios-red active:bg-ios-red/10 transition-colors shrink-0",
                                            onclick: move |_| {
                                                let _ = peers::unpair(&db(), &peer_id);
                                                peers_version += 1;
                                            },
                                            {t(&lang, "sync-unpair")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
