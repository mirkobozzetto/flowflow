// Share card (proposal 0001 T15) shown under a note or thread the user
// published, plus the frozen provenance card a KEPT note carries. Mockup
// section 3: no code ever displayed, actions = Copy link / Revoke.

use crate::application::i18n::{t, t_args};
use crate::application::sharing::{self, ShareError};
use crate::domain::share::{share_link, ShareKind, PROVENANCE_GONE};
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn expiry_label(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%d/%m/%Y")
                .to_string()
        })
        .unwrap_or_else(|_| iso.chars().take(10).collect())
}

fn share_error_key(e: &ShareError) -> &'static str {
    match e {
        ShareError::Refused => "share-refused",
        ShareError::NoBackend => "share-offline",
        ShareError::Gone => "share-not-found",
        ShareError::Other(_) => "share-offline",
    }
}

// The publish/copy/revoke card for one source (a note or an app thread).
#[component]
pub fn ShareSection(
    source_id: ReadSignal<String>,
    kind_thread: bool,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut version = use_signal(|| 0u32);
    let mut busy = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut link_copied = use_signal(|| false);

    // A menu asked to share this source: scroll the card into view, publish
    // if needed, copy the link, flash "Copié". Errors land in the card,
    // which is now on screen.
    let effect_kind = if kind_thread {
        ShareKind::Thread
    } else {
        ShareKind::Note
    };
    let effect_lang = lang.clone();
    use_effect(move || {
        let requested = (app.share_request)();
        if requested.as_deref() != Some(source_id().as_str()) {
            return;
        }
        app.share_request.set(None);
        dioxus::document::eval(&format!(
            "document.getElementById('share-section-{}')\
             ?.scrollIntoView({{behavior:'smooth', block:'center'}});",
            source_id()
        ));
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        let id = source_id();
        let kind = effect_kind.clone();
        let lang = effect_lang.clone();
        spawn(async move {
            let database = db();
            let res = match database.get_share(&id) {
                Some(existing) => Ok(existing),
                None => match kind {
                    ShareKind::Thread => {
                        sharing::publish_thread(&database, &id).await
                    }
                    ShareKind::Note => {
                        sharing::publish_note(&database, &id).await
                    }
                },
            };
            match res {
                Ok(s) => {
                    crate::ui::clipboard::copy_text(&share_link(&s.code));
                    link_copied.set(true);
                    spawn(async move {
                        futures_timer::Delay::new(
                            std::time::Duration::from_millis(2000),
                        )
                        .await;
                        link_copied.set(false);
                    });
                }
                Err(e) => error.set(Some(t(&lang, share_error_key(&e)))),
            }
            busy.set(false);
            version.set(version() + 1);
        });
    });

    let share = {
        let _ = version();
        let id = source_id();
        if id.is_empty() {
            None
        } else {
            db().get_share(&id)
        }
    };
    if source_id().is_empty() {
        return rsx! {};
    }

    let kind = if kind_thread {
        ShareKind::Thread
    } else {
        ShareKind::Note
    };
    let publish_key = if kind_thread {
        "share-publish-thread"
    } else {
        "share-publish-note"
    };

    rsx! {
        div { class: "mt-3", id: "share-section-{source_id}",
            if let Some(s) = share {
                div { class: "rounded-xl border border-stone-200 bg-stone-50 p-3",
                    p { class: "text-[10px] font-semibold uppercase tracking-wide text-ios-orange-dark mb-1",
                        {t(&lang, "share-active")}
                    }
                    p { class: "text-xs text-stone-600 leading-relaxed mb-2.5",
                        {t_args(&lang, "share-active-hint", &[("date", &expiry_label(&s.expires_at))])}
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "flex-1 h-[34px] flex items-center justify-center rounded-full bg-ios-orange text-white text-xs font-semibold active:opacity-80 transition-opacity",
                            onclick: {
                                let code = s.code.clone();
                                move |_| {
                                    crate::ui::clipboard::copy_text(&share_link(&code));
                                    link_copied.set(true);
                                    spawn(async move {
                                        futures_timer::Delay::new(
                                            std::time::Duration::from_millis(1500),
                                        )
                                        .await;
                                        link_copied.set(false);
                                    });
                                }
                            },
                            if link_copied() {
                                {t(&lang, "share-copied")}
                            } else {
                                {t(&lang, "share-copy-link")}
                            }
                        }
                        button {
                            class: "flex-1 h-[34px] flex items-center justify-center rounded-full border border-ios-red/30 text-ios-red text-xs font-medium active:bg-ios-red/10 transition-colors disabled:opacity-45",
                            disabled: busy(),
                            onclick: {
                                let lang = lang.clone();
                                move |_| {
                                    if busy() { return; }
                                    busy.set(true);
                                    error.set(None);
                                    let id = source_id();
                                    let lang = lang.clone();
                                    spawn(async move {
                                        let database = db();
                                        if let Err(e) = sharing::revoke(&database, &id).await {
                                            error.set(Some(t(&lang, share_error_key(&e))));
                                        }
                                        busy.set(false);
                                        version.set(version() + 1);
                                    });
                                }
                            },
                            {t(&lang, "share-revoke")}
                        }
                    }
                }
            } else {
                button {
                    class: "w-full h-[34px] flex items-center justify-center rounded-full border border-ios-orange/25 text-ios-orange-dark text-xs font-medium active:bg-ios-orange/10 transition-colors disabled:opacity-45",
                    disabled: busy(),
                    onclick: {
                        let lang = lang.clone();
                        move |_| {
                            if busy() { return; }
                            busy.set(true);
                            error.set(None);
                            let id = source_id();
                            let kind = kind.clone();
                            let lang = lang.clone();
                            spawn(async move {
                                let database = db();
                                let res = match kind {
                                    ShareKind::Thread => {
                                        sharing::publish_thread(&database, &id).await
                                    }
                                    ShareKind::Note => {
                                        sharing::publish_note(&database, &id).await
                                    }
                                };
                                if let Err(e) = res {
                                    error.set(Some(t(&lang, share_error_key(&e))));
                                }
                                busy.set(false);
                                version.set(version() + 1);
                            });
                        }
                    },
                    if busy() {
                        {t(&lang, "share-publishing")}
                    } else {
                        {t(&lang, publish_key)}
                    }
                }
            }
            if let Some(msg) = error() {
                p { class: "text-xs text-ios-red mt-1.5 px-1", "{msg}" }
            }
        }
    }
}

// Frozen origin of a kept note: author + capture date; greyed when the
// source no longer answers (author gone / share dead).
#[component]
pub fn ProvenanceCard(note_id: ReadSignal<String>) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let Some(p) = ({
        let id = note_id();
        if id.is_empty() {
            None
        } else {
            db().get_provenance(&id)
        }
    }) else {
        return rsx! {};
    };

    let gone = p.state == PROVENANCE_GONE;
    let name = p
        .author_name
        .clone()
        .unwrap_or_else(|| t(&lang, "share-author-anonymous"));
    let label = t_args(
        &lang,
        "share-by",
        &[
            ("name", name.as_str()),
            ("date", &expiry_label(&p.captured_at)),
        ],
    );

    rsx! {
        div {
            class: if gone {
                "flex items-center gap-2 mt-3 px-3 py-2 rounded-xl border border-stone-200 bg-stone-50 opacity-50"
            } else {
                "flex items-center gap-2 mt-3 px-3 py-2 rounded-xl border border-stone-200 bg-stone-50"
            },
            span { class: "shrink-0 w-[18px] h-[18px] rounded-full bg-ios-orange/15 flex items-center justify-center text-[9px] font-semibold text-ios-orange-dark",
                {name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}
            }
            p { class: "text-xs text-stone-600 truncate", "{label}" }
            if gone {
                span { class: "ml-auto shrink-0 text-[10px] text-stone-400 italic",
                    {t(&lang, "share-author-gone")}
                }
            }
        }
    }
}
