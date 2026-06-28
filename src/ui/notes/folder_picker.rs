use crate::application::i18n::t;
use crate::domain::{ChatScope, Folder};
use crate::infrastructure::persistence::Database;
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

    let all_folders: Memo<Vec<Folder>> = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    });
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

    use_effect(move || {
        let v = (app.picker_kb_down)();
        if v == *last_down.peek() {
            return;
        }
        last_down.set(v);
        let count = all_folders.peek().len() + 1;
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
            match all_folders.peek().get(i - 1) {
                Some(f) => Some(f.id.clone()),
                None => return,
            }
        };
        let mut go = choose;
        go(value);
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
                        class: "w-full text-left px-3 py-2.5 text-sm border-b border-stone-100 transition-colors duration-150",
                        class: if is_picked_all || (is_current_all && picked().is_none()) {
                            "text-ios-orange-dark font-medium bg-ios-orange-50"
                        } else if kb_index() == Some(0) {
                            "text-stone-500 bg-stone-100"
                        } else {
                            "text-stone-500 active:bg-stone-50 hover:bg-stone-50"
                        },
                        onclick: move |_| choose(None),
                        {t(&lang, "folder-picker-all")}
                    }
                }
            }
            for (idx, folder) in all_folders().into_iter().enumerate() {
                {
                    let fid = folder.id.clone();
                    let fname = folder.name.clone();
                    let is_current = selected() == Some(fid.clone());
                    let is_picked = picked() == Some(Some(fid.clone()));
                    rsx! {
                        button {
                            class: "w-full text-left px-3 py-2.5 text-sm transition-colors duration-150",
                            class: if is_picked || (is_current && picked().is_none()) {
                                "text-ios-orange-dark font-medium bg-ios-orange-50"
                            } else if kb_index() == Some(idx + 1) {
                                "text-stone-900 bg-stone-100"
                            } else {
                                "text-stone-900 active:bg-stone-50 hover:bg-stone-50"
                            },
                            onclick: move |_| choose(Some(fid.clone())),
                            "{fname}"
                        }
                    }
                }
            }
        }
    }
}
