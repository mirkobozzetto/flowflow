use crate::db::Database;
use crate::services::i18n::t;
use crate::services::sync::deeplink;
use crate::services::sync::peers::{self, PairingHost, PairingStatus};
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
            div {
                h2 { class: "text-lg font-semibold text-stone-900 mb-1",
                    {t(&lang, "sync-show-code")}
                }
                p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                    {t(&lang, "sync-show-code-hint")}
                }
                if let Some(h) = host() {
                    div { class: "space-y-3",
                        div {
                            class: "bg-white rounded-xl p-3 flex justify-center [&_svg]:max-w-full [&_svg]:h-auto",
                            dangerous_inner_html: "{h.qr_svg}",
                        }
                        p { class: "text-xs text-stone-500 text-center",
                            "{h.addr}:{h.port}"
                        }
                        p {
                            class: "text-[10px] text-stone-400 break-all px-2",
                            style: "user-select: all; -webkit-user-select: all;",
                            "{h.uri}"
                        }
                        match host_status() {
                            Some(PairingStatus::Paired { device_id }) => rsx! {
                                p { class: "text-sm text-ios-green text-center font-medium",
                                    {format!("{} {}", t(&lang, "sync-paired-with"), device_id)}
                                }
                            },
                            Some(PairingStatus::Failed(e)) => rsx! {
                                p { class: "text-sm text-ios-red text-center",
                                    {format!("{}: {}", t(&lang, "sync-failed"), e)}
                                }
                            },
                            Some(PairingStatus::Cancelled) => rsx! {
                                p { class: "text-sm text-stone-400 text-center",
                                    {t(&lang, "sync-cancelled")}
                                }
                            },
                            _ => rsx! {
                                p { class: "text-sm text-stone-500 text-center animate-pulse",
                                    {t(&lang, "sync-waiting")}
                                }
                            },
                        }
                    }
                } else {
                    button {
                        class: "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-orange text-white",
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
                                    host_status.set(Some(PairingStatus::Failed(e.to_string())));
                                    host.set(None);
                                }
                            }
                        },
                        {t(&lang, "sync-show-code")}
                    }
                }
                if host().is_none() {
                    if let Some(PairingStatus::Failed(e)) = host_status() {
                        p { class: "text-sm text-ios-red text-center mt-2",
                            {format!("{}: {}", t(&lang, "sync-failed"), e)}
                        }
                    }
                }
            }

            div { class: "border-t border-stone-200 pt-4",
                h2 { class: "text-lg font-semibold text-stone-900 mb-1",
                    {t(&lang, "sync-join-title")}
                }
                if scanned() {
                    div { class: "bg-ios-orange/10 border border-ios-orange/40 rounded-xl px-3 py-2.5 mb-3",
                        p { class: "text-sm text-stone-900 font-medium",
                            {t(&lang, "sync-scanned-title")}
                        }
                        p { class: "text-xs text-stone-600 leading-relaxed mt-0.5",
                            {t(&lang, "sync-scanned-hint")}
                        }
                    }
                } else {
                    p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                        {t(&lang, "sync-join-hint")}
                    }
                }
                textarea {
                    class: "w-full border border-stone-200 rounded-xl px-3 py-2.5 text-xs outline-none text-stone-900 bg-warm-white min-h-[72px]",
                    placeholder: "flowflow://pair#...",
                    value: "{join_uri}",
                    oninput: move |evt| join_uri.set(evt.value()),
                }
                button {
                    class: if joining() {
                        "w-full py-2.5 rounded-xl text-sm font-medium bg-stone-300 text-white mt-2"
                    } else {
                        "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-orange text-white mt-2"
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
                        p { class: "text-sm text-ios-green text-center mt-2 font-medium",
                            {format!("{} {}", t(&lang, "sync-paired-with"), id)}
                        }
                    },
                    Some(Err(e)) => rsx! {
                        p { class: "text-sm text-ios-red text-center mt-2",
                            {format!("{}: {}", t(&lang, "sync-failed"), e)}
                        }
                    },
                    None => rsx! {},
                }
            }

            div { class: "border-t border-stone-200 pt-4",
                h2 { class: "text-lg font-semibold text-stone-900 mb-3",
                    {t(&lang, "sync-peers-title")}
                }
                if peers_list().is_empty() {
                    p { class: "text-xs text-stone-400 text-center",
                        {t(&lang, "sync-no-peers")}
                    }
                } else {
                    div { class: "space-y-2",
                        for peer in peers_list() {
                            {
                                let peer_id = peer.device_id.clone();
                                rsx! {
                                    div {
                                        class: "flex items-center justify-between bg-warm-white border border-stone-200 rounded-xl px-3 py-2.5",
                                        div { class: "min-w-0",
                                            p { class: "text-sm text-stone-900 truncate", "{peer.device_id}" }
                                            if let Some(ref at) = peer.paired_at {
                                                p { class: "text-xs text-stone-500", "{at}" }
                                            }
                                        }
                                        button {
                                            class: "text-xs text-ios-red font-medium min-h-[44px] px-2 shrink-0",
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
