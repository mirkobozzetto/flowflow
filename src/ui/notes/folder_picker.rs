use crate::application::i18n::t;
use crate::domain::{folder_navigation_rows, ChatScope};
use crate::infrastructure::persistence::Database;
use crate::ui::folder_navigation::{set_folder_expanded, FolderSearch};
use crate::ui::icons::IconCaretRight;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ChatScopePicker() -> Element {
    let mut app: AppState = use_context();
    let highlight = use_signal(|| match (app.chat_scope)() {
        Some(ChatScope::Folder(id)) => Some(id),
        _ => None,
    });
    rsx! {
        FolderPicker {
            selected: highlight,
            on_pick: move |v: Option<String>| {
                app.chat_scope.set(v.map(ChatScope::Folder));
            },
        }
    }
}

#[component]
pub fn FolderPicker(
    selected: Signal<Option<String>>,
    on_pick: EventHandler<Option<String>>,
) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut picked: Signal<Option<Option<String>>> = use_signal(|| None);
    let mut closing = use_signal(|| false);
    let query = use_signal(String::new);
    let all_folders = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    });
    let spaces = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_spaces().unwrap_or_default()
    });
    let rows = use_memo(move || {
        folder_navigation_rows(
            &all_folders(),
            &(app.collapsed_folders)(),
            &query(),
        )
    });
    let searching = !query().trim().is_empty();
    let lang = (app.current_lang)();

    let mut choose = move |value: Option<String>| {
        if picked.peek().is_some() {
            return;
        }
        picked.set(Some(value.clone()));
        let mut selected = selected;
        spawn(async move {
            futures_timer::Delay::new(std::time::Duration::from_millis(120))
                .await;
            closing.set(true);
            futures_timer::Delay::new(std::time::Duration::from_millis(140))
                .await;
            selected.set(value.clone());
            on_pick.call(value);
            app.show_folder_picker.set(false);
        });
    };

    let mut kb_index: Signal<Option<usize>> = use_signal(|| None);
    let mut last_up = use_signal(|| *app.picker_kb_up.peek());
    let mut last_down = use_signal(|| *app.picker_kb_down.peek());
    let mut last_commit = use_signal(|| *app.picker_kb_commit.peek());

    // Filtering/folding changes the keyboard's destinations, not the selected
    // folder. Never let Enter pick a stale, now-hidden row.
    use_effect(move || {
        let _ = rows();
        kb_index.set(None);
    });
    use_effect(move || {
        let v = (app.picker_kb_down)();
        if v == *last_down.peek() {
            return;
        }
        last_down.set(v);
        let count = rows.peek().len() + 1;
        let next = match *kb_index.peek() {
            None => 0,
            Some(i) => (i + 1).min(count.saturating_sub(1)),
        };
        kb_index.set(Some(next));
    });
    use_effect(move || {
        let v = (app.picker_kb_up)();
        if v == *last_up.peek() {
            return;
        }
        last_up.set(v);
        let next = match *kb_index.peek() {
            None => 0,
            Some(i) => i.saturating_sub(1),
        };
        kb_index.set(Some(next));
    });
    use_effect(move || {
        let v = (app.picker_kb_commit)();
        if v == *last_commit.peek() {
            return;
        }
        last_commit.set(v);
        let Some(i) = *kb_index.peek() else {
            return;
        };
        let value = if i == 0 {
            None
        } else {
            match rows.peek().get(i - 1) {
                Some(row) => Some(row.folder.id.clone()),
                None => return,
            }
        };
        let mut go = choose;
        go(value);
    });
    use_effect(move || {
        if kb_index().is_some() {
            dioxus::document::eval(
                "requestAnimationFrame(() => document.querySelector('[data-folder-kb=\"true\"]')?.scrollIntoView({block: 'nearest'}));",
            );
        }
    });

    rsx! {
        div {
            class: "absolute left-0 right-0 top-0 z-20 bg-warm-white border-b border-stone-200 max-h-[60vh] overflow-y-auto shadow-md",
            class: if closing() {
                "animate-[slideUp_140ms_ease-in_forwards]"
            } else {
                "animate-[slideDown_150ms_ease-out]"
            },
            {
                let is_current_all = selected().is_none();
                let is_picked_all = picked() == Some(None);
                rsx! {
                    button {
                        class: "w-full shrink-0 min-h-[44px] text-left px-3 py-2.5 text-sm border-b border-stone-100 transition-colors duration-150 focus-visible:outline-stone-500",
                        class: if is_picked_all || (is_current_all && picked().is_none()) {
                            "text-ios-orange-dark font-medium bg-ios-orange-50"
                        } else if kb_index() == Some(0) {
                            "text-stone-500 bg-stone-100"
                        } else {
                            "text-stone-500 active:bg-stone-50 hover:bg-stone-50"
                        },
                        "data-folder-kb": kb_index() == Some(0),
                        onclick: move |_| choose(None),
                        {t(&lang, "folder-picker-all")}
                    }
                }
            }
            div { class: "p-3 shrink-0",
                FolderSearch {
                    query,
                    onkeydown: move |evt: KeyboardEvent| {
                        match evt.key() {
                            Key::ArrowDown => {
                                evt.prevent_default();
                                evt.stop_propagation();
                                app.picker_kb_down += 1;
                            }
                            Key::ArrowUp => {
                                evt.prevent_default();
                                evt.stop_propagation();
                                app.picker_kb_up += 1;
                            }
                            Key::Enter => {
                                evt.prevent_default();
                                evt.stop_propagation();
                                app.picker_kb_commit += 1;
                            }
                            Key::Escape => {
                                evt.stop_propagation();
                                app.show_folder_picker.set(false);
                            }
                            _ => {}
                        }
                    },
                }
            }
            div { class: "pb-2",
                if rows().is_empty() && searching {
                    p { class: "px-3 py-4 text-sm text-stone-500", role: "status",
                        {t(&lang, "folder-search-empty")}
                    }
                }
                for (idx, row) in rows().into_iter().enumerate() {
                    {
                        let fid = row.folder.id.clone();
                        let toggle_id = fid.clone();
                        let is_current = selected() == Some(fid.clone());
                        let is_picked = picked() == Some(Some(fid.clone()));
                        let expanded = !(app.collapsed_folders)().contains(&fid);
                        let indent = format!("padding-left: {}px", 8 + row.depth * 16);
                        let group = row.folder.space_id.as_ref().and_then(|id| {
                            spaces().into_iter().find(|s| &s.id == id).map(|s| s.name)
                        }).unwrap_or_else(|| t(&lang, "sidebar-folders-title"));
                        let path = if row.path.is_empty() { group } else { format!("{group} › {}", row.path) };
                        rsx! {
                            div {
                                key: "{row.folder.id}",
                                class: "flex items-center text-sm transition-colors duration-150",
                                class: if is_picked || (is_current && picked().is_none()) {
                                    "text-ios-orange-dark font-medium bg-ios-orange-50"
                                } else if kb_index() == Some(idx + 1) {
                                    "text-stone-900 bg-stone-100"
                                } else { "text-stone-900" },
                                style: "{indent}",
                                "data-folder-kb": kb_index() == Some(idx + 1),
                                if !searching {
                                    if row.has_children {
                                        button {
                                            class: "w-7 min-h-[44px] shrink-0 flex items-center justify-center text-stone-500 hover:opacity-70 focus-visible:outline-stone-500",
                                            "aria-expanded": expanded,
                                            "aria-label": format!("{} {}", t(&lang, if expanded { "folder-collapse" } else { "folder-expand" }), row.folder.name),
                                            onclick: move |_| set_folder_expanded(app, &db(), &toggle_id, !expanded),
                                            span { class: if expanded { "rotate-90" } else { "rotate-0" },
                                                IconCaretRight { size: 14 }
                                            }
                                        }
                                    } else { span { class: "w-7 shrink-0" } }
                                }
                                button {
                                    class: "flex-1 min-w-0 min-h-[44px] text-left px-2 py-2.5 active:bg-stone-50 hover:bg-stone-50 focus-visible:outline-stone-500",
                                    onclick: move |_| choose(Some(fid.clone())),
                                    span { class: "block [overflow-wrap:anywhere]", "{row.folder.name}" }
                                    if searching {
                                        span { class: "block text-xs text-stone-500 font-normal [overflow-wrap:anywhere]", "{path}" }
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
