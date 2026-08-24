use crate::application::i18n::t;
use crate::domain::flatten_tree;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::ui::icons::{IconArrowUpRight, IconCopy, IconFolder, IconTrash};
use crate::ui::{kit, AppState, RowMenu};
use dioxus::prelude::*;
use std::sync::Arc;

/// Panel width, mirroring `w-60` in `MENU_PANEL_ANCHORED`.
const MENU_W: u32 = 240;
/// Gap kept between the panel and the screen edge.
const EDGE: u32 = 12;
/// How much of the panel stays on screen when the press is near the bottom.
const MIN_VISIBLE: u32 = 240;

/// Applies a folder move off the render pass, so the card is never torn down
/// in the same patch as the menu that triggered it.
fn move_note(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    engine: Signal<Arc<SyncEngine>>,
    note_id: String,
    target: Option<String>,
) {
    app.row_menu.set(None);
    spawn(async move {
        let database = db();
        for old in database.folders_for_note(&note_id).unwrap_or_default() {
            let _ = database.remove_note_from_folder(&note_id, &old.id);
        }
        if let Some(fid) = target {
            let _ = database.add_note_to_folder(&note_id, &fid);
        }
        engine.peek().schedule_debounced();
        app.notes_version.set((app.notes_version)() + 1);
    });
}

/// Long-press sheet for a note in the list. Mounted ONCE at the app root, not
/// per card: `#notes-scroll` carries a `transform`, which would contain a
/// `fixed` child, and a menu living in the row's subtree gets orphaned when
/// the row is deleted (the frozen-taps bug).
#[component]
pub fn NoteRowMenu() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<SyncEngine>> = use_context();
    let mut confirm_delete = use_signal(|| false);
    let mut moving = use_signal(|| false);
    let lang = (app.current_lang)();

    let Some(RowMenu::Note(note_id)) = (app.row_menu)() else {
        return rsx! {};
    };

    let _ = (app.notes_version)();
    let _ = (app.folders_version)();
    let Ok(Some(note)) = db().get_note(&note_id) else {
        return rsx! {};
    };
    let folders = flatten_tree(&db().list_all_folders().unwrap_or_default());
    let current_folder = db()
        .folders_for_note(&note_id)
        .ok()
        .and_then(|f| f.first().map(|f| f.id.clone()));

    let title = note
        .title
        .clone()
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));
    let copy_payload = format!("{title}\n\n{}", note.content);
    let id_root = note_id.clone();
    let id_delete = note_id.clone();

    // Anchored to the press point, clamped in pure CSS: `min()` keeps the panel
    // on screen near the right or bottom edge without measuring the viewport
    // from Rust, and the height follows from the same clamped top.
    let (x, y) = (app.row_menu_at)();
    let anchor = format!(
        "--mx: min({x}px, calc(100vw - {MENU_W}px - {EDGE}px)); \
         --my: min({y}px, calc(100vh - {MIN_VISIBLE}px)); \
         left: var(--mx); top: var(--my); \
         max-height: calc(100vh - var(--my) - {EDGE}px);"
    );

    rsx! {
        div {
            class: "fixed overflow-y-auto {kit::MENU_PANEL_ANCHORED}",
            style: "{anchor}",
            if moving() {
                button {
                    class: kit::MENU_ITEM,
                    onclick: {
                        let id = id_root.clone();
                        move |_| {
                            moving.set(false);
                            move_note(app, db, engine, id.clone(), None);
                        }
                    },
                    IconFolder { size: 16 }
                    {t(&lang, "note-menu-no-folder")}
                }
                for (folder, depth) in folders.into_iter() {
                    {
                        let fid = folder.id.clone();
                        let nid = note_id.clone();
                        let is_current = current_folder.as_deref() == Some(fid.as_str());
                        let indent = format!("padding-left: {}px", 12 + depth * 14);
                        rsx! {
                            button {
                                key: "{folder.id}",
                                class: "{kit::MENU_ITEM}",
                                class: if is_current { "text-ios-orange-dark font-medium" } else { "" },
                                style: "{indent}",
                                onclick: move |_| {
                                    moving.set(false);
                                    move_note(app, db, engine, nid.clone(), Some(fid.clone()));
                                },
                                IconFolder { size: 16 }
                                span { class: "truncate", "{folder.name}" }
                            }
                        }
                    }
                }
            } else if confirm_delete() {
                div { class: "px-3 py-2.5",
                    p { class: "text-sm font-semibold text-stone-900 mb-0.5",
                        {t(&lang, "note-menu-delete-title")}
                    }
                    p { class: "text-xs text-stone-500 mb-3",
                        {t(&lang, "note-menu-delete-warning")}
                    }
                    div { class: "flex gap-2",
                        button {
                            class: kit::CONFIRM_BTN_GHOST,
                            onclick: move |_| {
                                confirm_delete.set(false);
                                app.row_menu.set(None);
                            },
                            {t(&lang, "chat-menu-cancel")}
                        }
                        button {
                            class: kit::CONFIRM_BTN_DANGER,
                            onclick: move |_| {
                                // Close this render pass, delete on the next task: the card
                                // must never be torn down in the same patch as its own menu.
                                let id = id_delete.clone();
                                confirm_delete.set(false);
                                app.row_menu.set(None);
                                spawn(async move {
                                    crate::application::note_persistence::delete_note(&db(), &id);
                                    engine.peek().schedule_debounced();
                                    app.notes_version.set((app.notes_version)() + 1);
                                });
                            },
                            {t(&lang, "chat-menu-delete")}
                        }
                    }
                }
            } else {
                p { class: "px-3 pt-1.5 pb-2 text-xs text-stone-400 truncate", "{title}" }
                button {
                    class: kit::MENU_ITEM,
                    onclick: move |_| moving.set(true),
                    IconFolder { size: 16 }
                    {t(&lang, "folder-menu-move")}
                }
                button {
                    class: kit::MENU_ITEM,
                    onclick: {
                        let payload = copy_payload.clone();
                        move |_| {
                            crate::ui::clipboard::copy_text(&payload);
                            app.row_menu.set(None);
                        }
                    },
                    IconCopy { size: 16 }
                    {t(&lang, "note-menu-copy")}
                }
                button {
                    class: kit::MENU_ITEM,
                    onclick: {
                        let nid = note_id.clone();
                        move |_| {
                            app.row_menu.set(None);
                            app.share_request.set(Some(nid.clone()));
                            app.view.set(crate::ui::View::NoteDetail {
                                note_id: nid.clone(),
                            });
                        }
                    },
                    IconArrowUpRight { size: 16 }
                    {t(&lang, "share-publish-note")}
                }
                div { class: kit::MENU_SEP }
                button {
                    class: kit::MENU_ITEM_DANGER,
                    onclick: move |_| confirm_delete.set(true),
                    IconTrash { size: 16 }
                    {t(&lang, "note-menu-delete")}
                }
            }
        }
    }
}
