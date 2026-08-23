// Reader side of a share (proposal 0001 T16, mockup sections 3-4): open a
// code read-only, keep a note with provenance, append/edit/delete one's own
// notes in a fil, see tombstones and the deletion-alignment banner, report
// the share, block an author (App Store 1.2). All orchestration lives in
// application::sharing; this view only renders and delegates.

use crate::application::i18n::{t, t_args};
use crate::application::sharing::{self, AlignmentEvent, ShareError};
use crate::infrastructure::backend::shares::{RemoteSharedNote, RemoteThread};
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
enum Load {
    Loading,
    NotFound,
    Failed(String),
    Ready(RemoteThread),
}

#[component]
pub fn SharedThreadView(code: ReadSignal<String>) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut load = use_signal(|| Load::Loading);
    let mut version = use_signal(|| 0u32);
    let mut banner: Signal<Option<String>> = use_signal(|| None);
    let mut busy = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut composing = use_signal(|| false);
    let mut compose_text = use_signal(String::new);
    let editing: Signal<Option<(String, String)>> = use_signal(|| None);
    let kept_flash = use_signal(|| false);
    let mut reported = use_signal(|| false);
    let confirm_block: Signal<Option<String>> = use_signal(|| None);

    let banner_lang = lang.clone();
    use_effect(move || {
        let _ = version();
        let c = code();
        let l = banner_lang.clone();
        spawn(async move {
            let database = db();
            match sharing::open(&database, &c).await {
                Ok(thread) => {
                    // deletion alignment for THIS share's kept notes
                    let events = sharing::align_kept_content(&database).await;
                    if events.iter().any(|e| {
                        matches!(e, AlignmentEvent::RemovedByAuthor { .. })
                    }) {
                        banner.set(Some(t(&l, "share-align-banner")));
                    }
                    load.set(Load::Ready(thread));
                }
                Err(ShareError::Gone) => load.set(Load::NotFound),
                Err(e) => load.set(Load::Failed(e.to_string())),
            }
        });
    });

    rsx! {
        div { class: "flex-1 overflow-y-auto px-4 pt-4 safe-pb-32 lg:px-[max(1rem,calc((100%-48rem)/2))]",
            match load() {
                Load::Loading => rsx! {
                    div { class: "flex items-center justify-center h-[40vh]",
                        span { class: "inline-block w-5 h-5 border-2 border-ios-orange border-t-transparent rounded-full animate-spin" }
                    }
                },
                Load::NotFound => rsx! {
                    div { class: "flex flex-col items-center justify-center gap-2 h-[40vh] px-6",
                        p { class: "text-stone-500 text-sm text-center", {t(&lang, "share-not-found")} }
                    }
                },
                Load::Failed(msg) => rsx! {
                    div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3 mt-2",
                        p { class: "text-xs text-stone-600 break-words", "{msg}" }
                    }
                },
                Load::Ready(thread) => {
                    let is_thread = thread.kind == "thread";
                    let database = db();
                    let can_append = is_thread;
                    let owner_name = thread.owner.display_name.clone()
                        .unwrap_or_else(|| t(&lang, "share-author-anonymous"));
                    let shared_date = thread.created_at.get(..10).unwrap_or("").to_string();
                    let by_label = t_args(&lang, "share-by", &[
                        ("name", owner_name.as_str()),
                        ("date", shared_date.as_str()),
                    ]);
                    rsx! {
                        if let Some(msg) = banner() {
                            div { class: "mb-3 px-3 py-2 rounded-xl bg-ios-orange/10 border border-ios-orange/20",
                                p { class: "text-xs text-ios-orange-dark leading-relaxed", "{msg}" }
                            }
                        }
                        if let Some(msg) = error() {
                            div { class: "mb-3 px-3 py-2 rounded-xl bg-ios-red/5 border border-ios-red/30",
                                p { class: "text-xs text-stone-600 break-words", "{msg}" }
                            }
                        }
                        // owner card (mockup: a-prov)
                        div { class: "flex items-center gap-2 mb-4 px-3 py-2 rounded-xl border border-stone-200 bg-stone-50",
                            span { class: "shrink-0 w-[22px] h-[22px] rounded-full bg-ios-orange/15 flex items-center justify-center text-[10px] font-semibold text-ios-orange-dark",
                                {owner_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}
                            }
                            p { class: "text-xs text-stone-600 truncate", "{by_label}" }
                        }
                        for note in thread.notes.clone() {
                            if !sharing::is_blocked(&database, note.author.author_ref.as_deref()) {
                                SharedNode {
                                    key: "{note.id}",
                                    code: code(),
                                    thread: thread.clone(),
                                    note: note.clone(),
                                    is_thread,
                                    editing,
                                    busy,
                                    error,
                                    version,
                                    kept_flash,
                                    confirm_block,
                                }
                            }
                        }
                        if kept_flash() {
                            p { class: "text-xs text-ios-orange-dark text-center mt-1",
                                {t(&lang, "share-kept")}
                            }
                        }
                        // footer: append (fil only) + report
                        if can_append {
                            if composing() {
                                div { class: "mt-2 bg-warm-white border border-stone-200 rounded-xl p-3",
                                    textarea {
                                        class: "w-full min-h-[80px] text-sm resize-none outline-none text-stone-900",
                                        placeholder: t(&lang, "share-note-placeholder"),
                                        value: "{compose_text}",
                                        oninput: move |evt| compose_text.set(evt.value()),
                                    }
                                    div { class: "flex justify-end gap-2 mt-1",
                                        button {
                                            class: "h-8 px-3 rounded-lg bg-stone-100 text-stone-900 text-xs font-medium",
                                            onclick: move |_| {
                                                composing.set(false);
                                                compose_text.set(String::new());
                                            },
                                            {t(&lang, "share-cancel")}
                                        }
                                        button {
                                            class: "h-8 px-3 rounded-lg bg-ios-orange text-white text-xs font-medium disabled:opacity-45",
                                            disabled: busy() || compose_text().trim().is_empty(),
                                            onclick: move |_| {
                                                if busy() { return; }
                                                busy.set(true);
                                                error.set(None);
                                                let c = code();
                                                let body = compose_text();
                                                spawn(async move {
                                                    let database = db();
                                                    match sharing::append(&database, &c, None, &body).await {
                                                        Ok(_) => {
                                                            composing.set(false);
                                                            compose_text.set(String::new());
                                                        }
                                                        Err(e) => error.set(Some(share_err_label(&e))),
                                                    }
                                                    busy.set(false);
                                                    version.set(version() + 1);
                                                });
                                            },
                                            {t(&lang, "share-save")}
                                        }
                                    }
                                }
                            } else {
                                button {
                                    class: "w-full h-[38px] mt-2 flex items-center justify-center rounded-full border border-ios-orange/25 text-ios-orange-dark text-xs font-medium active:bg-ios-orange/10 transition-colors",
                                    onclick: move |_| composing.set(true),
                                    {t(&lang, "share-add-note")}
                                }
                            }
                        }
                        div { class: "flex justify-center mt-4",
                            button {
                                class: if reported() {
                                    "text-xs text-stone-400"
                                } else {
                                    "text-xs text-stone-400 underline underline-offset-2 active:text-stone-600"
                                },
                                disabled: reported() || busy(),
                                onclick: move |_| {
                                    if busy() { return; }
                                    busy.set(true);
                                    let c = code();
                                    spawn(async move {
                                        let database = db();
                                        if sharing::report(&database, &c).await.is_ok() {
                                            reported.set(true);
                                        }
                                        busy.set(false);
                                    });
                                },
                                if reported() {
                                    {t(&lang, "share-reported")}
                                } else {
                                    {t(&lang, "share-report")}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn share_err_label(e: &ShareError) -> String {
    e.to_string()
}

#[component]
#[allow(clippy::too_many_arguments)]
fn SharedNode(
    code: String,
    thread: RemoteThread,
    note: RemoteSharedNote,
    is_thread: bool,
    editing: Signal<Option<(String, String)>>,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
    version: Signal<u32>,
    kept_flash: Signal<bool>,
    confirm_block: Signal<Option<String>>,
) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let date = note.created_at.get(..10).unwrap_or("").to_string();
    let author_name = note
        .author
        .display_name
        .clone()
        .unwrap_or_else(|| t(&lang, "share-author-anonymous"));
    let title = note.title.clone().unwrap_or_default();
    let already_kept = db()
        .provenances_for_code(&code)
        .iter()
        .any(|p| p.remote_note_id == note.id);
    let is_editing = editing().map(|(id, _)| id == note.id).unwrap_or(false);

    rsx! {
        div { class: "flex gap-3 pb-4",
            div { class: "flex flex-col items-center pt-1.5",
                div {
                    class: if note.deleted {
                        "w-2.5 h-2.5 rounded-full bg-stone-300 shrink-0"
                    } else {
                        "w-2.5 h-2.5 rounded-full bg-ios-orange shrink-0"
                    },
                }
                div { class: "flex-1 w-px bg-stone-200 mt-1" }
            }
            div {
                class: if note.deleted {
                    "flex-1 min-w-0 bg-stone-50 border border-dashed border-stone-200 rounded-xl p-3"
                } else {
                    "flex-1 min-w-0 bg-warm-white border border-stone-200 rounded-xl p-3"
                },
                if note.deleted {
                    p { class: "text-sm text-stone-400 italic",
                        {t(&lang, "share-tombstone")}
                    }
                } else if is_editing {
                    textarea {
                        class: "w-full min-h-[80px] text-sm resize-none outline-none text-stone-900",
                        value: editing().map(|(_, v)| v).unwrap_or_default(),
                        oninput: move |evt| {
                            if let Some((id, _)) = editing() {
                                editing.set(Some((id, evt.value())));
                            }
                        },
                    }
                    div { class: "flex justify-end gap-2 mt-1",
                        button {
                            class: "h-8 px-3 rounded-lg bg-stone-100 text-stone-900 text-xs font-medium",
                            onclick: move |_| editing.set(None),
                            {t(&lang, "share-cancel")}
                        }
                        button {
                            class: "h-8 px-3 rounded-lg bg-ios-orange text-white text-xs font-medium disabled:opacity-45",
                            disabled: busy(),
                            onclick: {
                                let c = code.clone();
                                let nid = note.id.clone();
                                let ntitle = note.title.clone();
                                move |_| {
                                    if busy() { return; }
                                    let Some((_, body)) = editing() else { return; };
                                    busy.set(true);
                                    error.set(None);
                                    let c = c.clone();
                                    let nid = nid.clone();
                                    let ntitle = ntitle.clone();
                                    spawn(async move {
                                        let database = db();
                                        match sharing::update_own(
                                            &database, &c, &nid, ntitle.as_deref(), &body,
                                        )
                                        .await
                                        {
                                            Ok(()) => editing.set(None),
                                            Err(e) => error.set(Some(share_err_label(&e))),
                                        }
                                        busy.set(false);
                                        version.set(version() + 1);
                                    });
                                }
                            },
                            {t(&lang, "share-save")}
                        }
                    }
                } else {
                    div { class: "flex justify-between items-baseline mb-1 gap-2",
                        if !title.is_empty() {
                            h4 { class: "font-medium text-sm text-stone-900", "{title}" }
                        }
                        span { class: "text-xs text-stone-400 shrink-0 ml-auto", "{date}" }
                    }
                    if let Some(ref body) = note.content {
                        p { class: "text-stone-700 text-sm whitespace-pre-wrap break-words",
                            "{body}"
                        }
                    }
                    // author row (mockup: a-who) + own actions / keep / block
                    div { class: "flex items-center gap-1.5 mt-2 text-xs text-stone-500",
                        span { class: "shrink-0 w-[15px] h-[15px] rounded-full bg-ios-orange/15 flex items-center justify-center text-[8px] font-semibold text-ios-orange-dark",
                            {author_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}
                        }
                        span { class: "font-semibold text-stone-700 truncate", "{author_name}" }
                        div { class: "ml-auto flex items-center gap-3 shrink-0",
                            if note.own {
                                button {
                                    class: "text-ios-orange-dark font-semibold",
                                    onclick: {
                                        let nid = note.id.clone();
                                        let body = note.content.clone().unwrap_or_default();
                                        move |_| editing.set(Some((nid.clone(), body.clone())))
                                    },
                                    {t(&lang, "share-edit")}
                                }
                                button {
                                    class: "text-ios-red font-semibold",
                                    disabled: busy(),
                                    onclick: {
                                        let c = code.clone();
                                        let nid = note.id.clone();
                                        move |_| {
                                            if busy() { return; }
                                            busy.set(true);
                                            error.set(None);
                                            let c = c.clone();
                                            let nid = nid.clone();
                                            spawn(async move {
                                                let database = db();
                                                if let Err(e) =
                                                    sharing::delete_own(&database, &c, &nid).await
                                                {
                                                    error.set(Some(share_err_label(&e)));
                                                }
                                                busy.set(false);
                                                version.set(version() + 1);
                                            });
                                        }
                                    },
                                    {t(&lang, "share-delete")}
                                }
                            } else {
                                if already_kept {
                                    span { class: "text-stone-400", {t(&lang, "share-kept")} }
                                } else {
                                    button {
                                        class: "text-ios-orange-dark font-semibold",
                                        onclick: {
                                            let thread = thread.clone();
                                            let note = note.clone();
                                            move |_| {
                                                let database = db();
                                                if sharing::keep_note(&database, &thread, &note).is_ok() {
                                                    kept_flash.set(true);
                                                    version.set(version() + 1);
                                                }
                                            }
                                        },
                                        {t(&lang, "share-keep")}
                                    }
                                }
                                if let Some(ref aref) = note.author.author_ref {
                                    if confirm_block().as_deref() == Some(aref.as_str()) {
                                        button {
                                            class: "text-ios-red font-semibold",
                                            onclick: {
                                                let aref = aref.clone();
                                                move |_| {
                                                    sharing::block_author(&db(), &aref);
                                                    confirm_block.set(None);
                                                    version.set(version() + 1);
                                                }
                                            },
                                            {t(&lang, "share-block-author")} " ?"
                                        }
                                    } else if !note.own {
                                        button {
                                            class: "text-stone-400",
                                            onclick: {
                                                let aref = aref.clone();
                                                move |_| confirm_block.set(Some(aref.clone()))
                                            },
                                            {t(&lang, "share-block-author")}
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
