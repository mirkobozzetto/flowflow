use crate::application::i18n::t;
use crate::application::mention::{search_mentions, MentionHit};
use crate::domain::ChatScope;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::tools_menu::{LEAD_ICON, ROW, ROW_TITLE, SECTION, SEP};
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

const MENTION_CAP: usize = 8;

#[derive(Clone, PartialEq)]
pub struct MentionedNote {
    pub note_id: String,
    pub title: String,
}

// Mentions are detected at the end of the input ("... @frag"). Selecting an item
// replaces that trailing "@frag" with the chosen token (or removes it for a tool).
fn replace_trailing_at(s: &str, replacement: &str) -> String {
    match s.rfind('@') {
        Some(at) => format!("{}{}", &s[..at], replacement),
        None => format!("{s}{replacement}"),
    }
}

fn scoped_note_ids(
    db: &Database,
    scope: &Option<ChatScope>,
) -> Option<HashSet<String>> {
    match scope {
        Some(ChatScope::Folder(fid)) => Some(
            db.list_notes_in_folder_tree(fid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect(),
        ),
        Some(ChatScope::Thread(tid)) => Some(
            db.list_thread_notes(tid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect(),
        ),
        None => None,
    }
}

#[component]
pub fn MentionMenu(
    input: Signal<String>,
    mentions: Signal<Vec<MentionedNote>>,
    query: String,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let scope = (app.chat_scope)();
    let (results, folder_of): (
        crate::application::mention::MentionResults,
        std::collections::HashMap<String, String>,
    ) = {
        let db = db();
        let notes = db.list_notes().unwrap_or_default();
        let scoped = scoped_note_ids(&db, &scope);
        let results =
            search_mentions(&notes, &query, scoped.as_ref(), MENTION_CAP);
        // Folder subtitle only for the handful of rows actually shown.
        let folder_of = results
            .in_scope
            .iter()
            .chain(results.others.iter())
            .filter_map(|h| {
                db.folders_for_note(&h.note_id)
                    .ok()
                    .and_then(|fs| fs.into_iter().next())
                    .map(|f| (h.note_id.clone(), f.name))
            })
            .collect();
        (results, folder_of)
    };

    let scoped_chat = scope.is_some();
    let others_label = if scoped_chat {
        t(&lang, "chat-mention-others")
    } else if query.is_empty() {
        t(&lang, "chat-mention-recent")
    } else {
        t(&lang, "chat-mention-section-notes")
    };
    let empty = results.in_scope.is_empty() && results.others.is_empty();

    rsx! {
        div { class: "absolute bottom-full left-2 right-2 lg:left-0 lg:right-auto lg:w-80 mb-2 z-40 max-h-72 overflow-y-auto bg-warm-white border border-stone-200 rounded-xl shadow-lg p-2 popover-pop",
            div { class: SECTION, {t(&lang, "chat-tools-section-tools")} }
            button {
                class: ROW,
                onclick: move |_| {
                    app.chat_web.set(true);
                    input.set(replace_trailing_at(&input(), ""));
                    app.show_mention_menu.set(false);
                },
                span { class: LEAD_ICON, IconGlobeSimple { size: 22 } }
                span { class: ROW_TITLE, {t(&lang, "chat-tools-web")} }
            }
            div { class: SEP }
            if empty {
                div { class: SECTION, {others_label.clone()} }
                div { class: "px-3 py-2 text-sm text-stone-400",
                    {t(&lang, "chat-mention-no-notes")}
                }
            }
            if !results.in_scope.is_empty() {
                div { class: SECTION, {t(&lang, "chat-mention-in-folder")} }
                for hit in results.in_scope.clone() {
                    MentionRow {
                        hit: hit.clone(),
                        folder: folder_of.get(&hit.note_id).cloned(),
                        input: input,
                        mentions: mentions,
                    }
                }
            }
            if !results.others.is_empty() {
                div { class: SECTION, {others_label} }
                for hit in results.others.clone() {
                    MentionRow {
                        hit: hit.clone(),
                        folder: folder_of.get(&hit.note_id).cloned(),
                        input: input,
                        mentions: mentions,
                    }
                }
            }
        }
    }
}

#[component]
fn MentionRow(
    hit: MentionHit,
    folder: Option<String>,
    input: Signal<String>,
    mentions: Signal<Vec<MentionedNote>>,
) -> Element {
    let mut app: AppState = use_context();
    let subtitle = match &folder {
        Some(name) => format!("{name} · {}", hit.date),
        None => hit.date.clone(),
    };
    let id = hit.note_id.clone();
    let label = hit.label.clone();
    rsx! {
        button {
            key: "{hit.note_id}",
            class: ROW,
            onclick: move |_| {
                let token = format!("@{label} ");
                input.set(replace_trailing_at(&input(), &token));
                let mut next = mentions();
                if !next.iter().any(|m| m.note_id == id) {
                    next.push(MentionedNote {
                        note_id: id.clone(),
                        title: label.clone(),
                    });
                }
                mentions.set(next);
                app.show_mention_menu.set(false);
            },
            span { class: LEAD_ICON, IconNote { size: 22 } }
            div { class: "flex-1 min-w-0 text-left",
                div {
                    class: "truncate {ROW_TITLE}",
                    class: if hit.untitled { "italic text-stone-500" },
                    "{hit.label}"
                }
                div { class: "truncate text-xs text-stone-400", "{subtitle}" }
            }
        }
    }
}
