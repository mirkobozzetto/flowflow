use crate::application::i18n::t;
use crate::application::space::{self, FolderRight, ShareTarget};
use crate::domain::space::MODE_READ;

// the server caps a space name at this length
const MAX_TEAM_NAME_CHARS: usize = 100;
use crate::domain::{
    flatten_tree, subtree_ids, Folder, NewFolder, UpdateFolder,
};
use crate::infrastructure::persistence::Database;
use crate::ui::delete_confirm::DeleteConfirm;
use crate::ui::icons::*;
use crate::ui::kit;
use crate::ui::{AppState, RowMenu, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn FolderSection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let lang = (app.current_lang)();

    // My own themes only: a shared theme is listed under its space, below.
    let folders = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_root_folders()
            .unwrap_or_default()
            .into_iter()
            .filter(|f| f.space_id.is_none())
            .collect::<Vec<_>>()
    });
    let spaces = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_spaces().unwrap_or_default()
    });

    rsx! {
        div { class: "flex items-center justify-between px-2 mb-2",
            span { class: kit::SECTION_LABEL, {t(&lang, "sidebar-folders-title")} }
            button {
                class: "w-11 h-11 flex items-center justify-center rounded-full transition-all duration-200",
                class: if creating() {
                    "rotate-45 text-ios-orange-dark bg-ios-orange-50"
                } else {
                    "text-stone-500 hover:bg-stone-100"
                },
                onclick: move |_| {
                    creating.set(!creating());
                    if creating() {
                        dioxus::document::eval(
                            r#"
                            requestAnimationFrame(function() {
                                var el = document.getElementById('new-theme-input');
                                if (el) el.focus();
                            });
                            "#,
                        );
                    }
                },
                IconPlus { size: 18 }
            }
        }
        div {
            class: "overflow-hidden transition-all duration-200 px-1",
            class: if creating() { "max-h-16 opacity-100 mb-2" } else { "max-h-0 opacity-0" },
            div { class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1",
                input {
                    id: "new-theme-input",
                    class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                    placeholder: t(&lang, "sidebar-folder-placeholder"),
                    value: "{new_name}",
                    oninput: move |evt| new_name.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Escape {
                            creating.set(false);
                        }
                    },
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !new_name().trim().is_empty() {
                            let folder = NewFolder {
                                name: new_name().trim().to_string(),
                                description: None,
                                parent_id: None,
                            };
                            let _ = db().create_folder(&folder);
                            new_name.set(String::new());
                            creating.set(false);
                            app.folders_version.set((app.folders_version)() + 1);
                        }
                    },
                }
                button {
                    class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                    class: if new_name().trim().is_empty() {
                        "text-stone-300"
                    } else {
                        "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                    },
                    onclick: move |_| {
                        if !new_name().trim().is_empty() {
                            let folder = NewFolder {
                                name: new_name().trim().to_string(),
                                description: None,
                                parent_id: None,
                            };
                            let _ = db().create_folder(&folder);
                            new_name.set(String::new());
                            creating.set(false);
                            app.folders_version.set((app.folders_version)() + 1);
                        }
                    },
                    IconCheck { size: 16 }
                }
            }
        }
        if folders().is_empty() && !creating() {
            div { class: "mx-2 py-4 flex flex-col items-center text-center",
                div { class: "w-12 h-12 rounded-xl bg-ios-orange-50 text-ios-orange flex items-center justify-center mb-2",
                    IconFolder { size: 22 }
                }
                p { class: "text-[13px] text-stone-500", {t(&lang, "sidebar-no-folders")} }
            }

        }
        for folder in folders() {
            FolderItem { key: "{folder.id}", folder: folder, depth: 0 }
        }
        if !spaces().is_empty() {
            div { class: "h-px bg-stone-200 my-3" }
            div { class: "flex items-center gap-1.5 px-2 mb-1 {kit::SECTION_LABEL}",
                IconUsersThree { size: 14 }
                {t(&lang, "sidebar-collab-title")}
            }
        }
        for (i, space) in spaces().into_iter().enumerate() {
            div { key: "{space.id}",
                if i > 0 {
                    div { class: "h-px bg-stone-200 my-2 ml-7" }
                }
                super::space_section::SpaceSection { space: space }
            }
        }
        super::join_link::JoinLink {}
    }
}

#[component]
pub(super) fn FolderItem(folder: Folder, depth: u32) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut creating_sub = use_signal(|| false);
    let mut sub_name = use_signal(String::new);
    let mut editing = use_signal(|| false);
    let mut edit_name = use_signal(|| folder.name.clone());
    let mut confirm_delete = use_signal(|| false);
    let mut moving = use_signal(|| false);
    let lang = (app.current_lang)();

    // One global menu slot (see RowMenu): the outside-click backdrop lives outside
    // this row's subtree, so deleting the row can never orphan it.
    let menu_open =
        (app.row_menu)() == Some(RowMenu::Folder(folder.id.clone()));

    let folder_id = folder.id.clone();
    let folder_id_menu = folder.id.clone();
    let folder_id_for_delete = folder.id.clone();
    let folder_id_for_sub = folder.id.clone();
    let folder_id_for_sub2 = folder.id.clone();
    let folder_id_for_rename = folder.id.clone();
    let folder_id_for_rename2 = folder.id.clone();
    let folder_id_for_toggle = folder.id.clone();
    let folder_id_for_count = folder.id.clone();
    let folder_id_for_move = folder.id.clone();
    let folder_id_move_root = folder.id.clone();
    let folder_id_move_self = folder.id.clone();
    let folder_id_for_badge = folder.id.clone();
    let folder_id_for_share = folder.id.clone();
    let mut sharing = use_signal(|| false);
    let mut share_open = use_signal(|| false);
    let mut share_name = use_signal(String::new);
    // lock by default: dropping a theme into a team opens its notes to every
    // member at once, writing is the choice to make explicit
    let mut share_collab = use_signal(|| false);
    let mut share_error = use_signal(|| None::<String>);
    let owned_spaces = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_spaces()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.is_owner)
            .collect::<Vec<_>>()
    });
    let mut fail = move |e: space::SpaceError| {
        eprintln!("[space] {e}");
        share_error.set(Some(t(&(app.current_lang)(), space::error_key(&e))))
    };
    let share = use_callback(move |target: ShareTarget| {
        let fid = folder_id_for_share.clone();
        let collab = share_collab();
        sharing.set(true);
        share_open.set(false);
        share_error.set(None);
        spawn(async move {
            if let Err(e) =
                space::share_existing_folder(&db(), &fid, target, collab).await
            {
                fail(e);
            }
            sharing.set(false);
            app.folders_version.set((app.folders_version)() + 1);
            app.notes_version.set((app.notes_version)() + 1);
        });
    });
    // A shared theme is renamed and deleted through its space: a local-only
    // change is overwritten by the next pull.
    let space_ref = folder.space_id.clone().zip(folder.remote_id.clone());
    let space_ref_delete = space_ref.clone();
    let folder_collab = folder.mode.as_deref() != Some(MODE_READ);
    let parent_local = folder.parent_id.clone();
    let rename =
        use_callback(move |(local_id, name): (String, String)| match space_ref
            .clone()
        {
            Some((space_id, remote_id)) => {
                let parent_remote = parent_local
                    .as_deref()
                    .and_then(|p| db().get_folder(p).ok().flatten())
                    .and_then(|f| f.remote_id);
                spawn(async move {
                    if let Err(e) = space::update_folder(
                        &db(),
                        &space_id,
                        &remote_id,
                        parent_remote.as_deref(),
                        &name,
                        folder_collab,
                    )
                    .await
                    {
                        fail(e);
                    }
                    app.folders_version.set((app.folders_version)() + 1);
                });
            }
            None => {
                let upd = UpdateFolder {
                    name: Some(name),
                    description: None,
                    parent_id: None,
                };
                let _ = db().update_folder(&local_id, &upd);
                app.folders_version.set((app.folders_version)() + 1);
            }
        });
    // A sub-theme of a shared theme is created in the space, never as a local
    // folder that the next pull would not know.
    let space_parent = folder.space_id.clone().zip(folder.remote_id.clone());
    let space_parent2 = space_parent.clone();
    let mut create_sub =
        move |name: String,
              parent_local: String,
              parent: Option<(String, String)>| {
            match parent {
                Some((space_id, remote_id)) => {
                    spawn(async move {
                        let database = db();
                        if let Err(e) =
                            crate::application::space::create_folder(
                                &database,
                                &space_id,
                                Some(&remote_id),
                                &name,
                                true,
                            )
                            .await
                        {
                            eprintln!("[space] sub-theme: {e}");
                        }
                        app.folders_version.set((app.folders_version)() + 1);
                    });
                }
                None => {
                    let folder = NewFolder {
                        name,
                        description: None,
                        parent_id: Some(parent_local.clone()),
                    };
                    let _ = db().create_folder(&folder);
                    app.folders_version.set((app.folders_version)() + 1);
                }
            }
            let mut set = (app.expanded_folders)();
            set.insert(parent_local);
            app.expanded_folders.set(set);
        };

    // Whether this theme lives in a shared space, and whether the user may add
    // notes to it. Same call the compose button uses, so the badge never
    // promises a write that would be refused.
    let right = use_memo(move || {
        let _v = (app.folders_version)();
        crate::application::space::folder_right(&db(), &folder_id_for_badge)
    });

    let children = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_subfolders(&folder_id).unwrap_or_default()
    });
    let has_children = !children().is_empty();

    // Expansion lives in AppState so the tree survives drawer close/reopen.
    let is_expanded = (app.expanded_folders)().contains(&folder.id);
    let toggle_expanded = use_callback(move |_: ()| {
        let mut set = (app.expanded_folders)();
        if !set.remove(&folder_id_for_toggle) {
            set.insert(folder_id_for_toggle.clone());
        }
        app.expanded_folders.set(set);
    });

    let note_count = use_memo(move || {
        let _f = (app.folders_version)();
        let _n = (app.notes_version)();
        db().count_notes_in_folder_tree(&folder_id_for_count)
            .unwrap_or(0)
    });

    // Move targets: every folder except this one and its own subtree (cycle guard).
    let move_targets = use_memo(move || {
        let _v = (app.folders_version)();
        let all = db().list_all_folders().unwrap_or_default();
        let forbidden = subtree_ids(&all, &folder_id_for_move);
        flatten_tree(&all)
            .into_iter()
            .filter(|(f, _)| !forbidden.contains(&f.id))
            .collect::<Vec<(Folder, u32)>>()
    });

    let folder_id_nav = folder.id.clone();
    let is_selected =
        (app.selected_folder_id)().as_deref() == Some(folder_id_nav.as_str());

    let margin = format!("margin-left: {}px", depth * 12);

    rsx! {
        div { style: "{margin}",
            if editing() {
                div {
                    class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 my-1",
                    style: "animation: popIn 0.16s ease-out;",
                    input {
                        class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900",
                        value: "{edit_name}",
                        oninput: move |evt| edit_name.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                editing.set(false);
                            }
                        },
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter && !edit_name().trim().is_empty() {
                                rename((folder_id_for_rename.clone(), edit_name().trim().to_string()));
                                editing.set(false);
                            }
                        },
                    }
                    button {
                        class: "w-10 h-10 shrink-0 flex items-center justify-center rounded-lg transition-colors duration-150",
                        class: if edit_name().trim().is_empty() {
                            "text-stone-300"
                        } else {
                            "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                        },
                        onclick: move |_| {
                            if !edit_name().trim().is_empty() {
                                rename((folder_id_for_rename2.clone(), edit_name().trim().to_string()));
                                editing.set(false);
                            }
                        },
                        IconCheck { size: 16 }
                    }
                    button {
                        class: "w-10 h-10 shrink-0 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors duration-150",
                        onclick: move |_| editing.set(false),
                        IconX { size: 14 }
                    }
                }
            } else {
                div { class: "flex items-center group relative",
                    if has_children {
                        button {
                            class: "min-w-[28px] min-h-[44px] flex items-center justify-center hover:opacity-70 transition-opacity duration-150",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                toggle_expanded(());
                            },
                            span {
                                class: "text-stone-400 transition-transform duration-150",
                                class: if is_expanded { "rotate-90" } else { "rotate-0" },
                                IconCaretRight { size: 14 }
                            }
                        }
                    } else {
                        div { class: "w-7 min-w-[28px]" }
                    }
                    button {
                        class: if is_selected {
                            "flex-1 flex items-center gap-2 text-left px-2 py-2.5 text-sm font-medium text-ios-orange-dark bg-ios-orange-50 rounded-lg min-h-[44px]"
                        } else {
                            "flex-1 flex items-center gap-2 text-left px-2 py-2.5 text-sm text-stone-900 rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150"
                        },
                        onclick: move |_| {
                            if has_children {
                                toggle_expanded(());
                            }
                            app.selected_folder_id.set(Some(folder_id_nav.clone()));
                            app.sidebar_open.set(false);
                            crate::ui::sidebar::navigate_with_slide(app, View::NotesList);
                        },
                        IconFolder { size: 16 }
                        span { class: "flex-1 min-w-0 truncate", "{folder.name}" }
                        if right() == FolderRight::SpaceReadOnly {
                            span { class: "shrink-0 text-stone-400", title: t(&lang, "space-badge-readonly"),
                                IconLockSimple { size: 14 }
                            }
                        }
                        if note_count() > 0 {
                            span { class: "shrink-0 text-xs text-stone-400 tabular-nums", "{note_count}" }
                        }
                    }
                    button {
                        class: "w-9 h-11 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-all duration-150",
                        class: if menu_open {
                            "text-stone-600"
                        } else {
                            "lg:opacity-0 lg:group-hover:opacity-100"
                        },
                        onclick: move |evt| {
                            evt.stop_propagation();
                            confirm_delete.set(false);
                            moving.set(false);
                            app.row_menu.set(if menu_open {
                                None
                            } else {
                                Some(RowMenu::Folder(folder_id_menu.clone()))
                            });
                        },
                        IconDotsThree { size: 20 }
                    }
                    if menu_open {
                        div {
                            id: "row-menu",
                            class: "absolute right-2 top-full max-h-64 overflow-y-auto {kit::MENU_PANEL}",
                            onclick: move |evt| evt.stop_propagation(),
                            if moving() {
                                if folder.parent_id.is_some() {
                                    button {
                                        class: kit::MENU_ITEM,
                                        onclick: move |_| {
                                            let upd = UpdateFolder {
                                                name: None,
                                                description: None,
                                                parent_id: Some(None),
                                            };
                                            let _ = db().update_folder(&folder_id_move_root, &upd);
                                            moving.set(false);
                                            app.row_menu.set(None);
                                            app.folders_version.set((app.folders_version)() + 1);
                                        },
                                        IconFolder { size: 16 }
                                        {t(&lang, "folder-menu-move-root")}
                                    }
                                }
                                for (target, target_depth) in move_targets() {
                                    {
                                        let tid = target.id.clone();
                                        let fid = folder_id_move_self.clone();
                                        let indent = format!("padding-left: {}px", 12 + target_depth * 14);
                                        rsx! {
                                            button {
                                                key: "{target.id}",
                                                class: kit::MENU_ITEM,
                                                style: "{indent}",
                                                onclick: move |_| {
                                                    let upd = UpdateFolder {
                                                        name: None,
                                                        description: None,
                                                        parent_id: Some(Some(tid.clone())),
                                                    };
                                                    let _ = db().update_folder(&fid, &upd);
                                                    moving.set(false);
                                                    app.row_menu.set(None);
                                                    app.folders_version.set((app.folders_version)() + 1);
                                                },
                                                IconFolder { size: 16 }
                                                span { class: "truncate", "{target.name}" }
                                            }
                                        }
                                    }
                                }
                            } else if confirm_delete() {
                                DeleteConfirm {
                                    title: t(&lang, "folder-menu-delete-title"),
                                    warning: t(&lang, "folder-menu-delete-warning"),
                                    cancel_label: t(&lang, "chat-menu-cancel"),
                                    confirm_label: t(&lang, "chat-menu-delete"),
                                    on_cancel: move |_| {
                                        confirm_delete.set(false);
                                        app.row_menu.set(None);
                                    },
                                    on_confirm: move |_| {
                                        // Close the menu THIS render pass, delete on the next
                                        // task: the row must never be torn down in the same
                                        // patch as its own open menu (orphaned-node freeze).
                                        confirm_delete.set(false);
                                        app.row_menu.set(None);
                                        let fid = folder_id_for_delete.clone();
                                        let space_ref = space_ref_delete.clone();
                                        spawn(async move {
                                            match space_ref {
                                                Some((space_id, remote_id)) => {
                                                    if let Err(e) = space::delete_folder(&db(), &space_id, &remote_id).await {
                                                        fail(e);
                                                    }
                                                }
                                                None => {
                                                    let _ = db().delete_folder(&fid);
                                                }
                                            }
                                            if (app.selected_folder_id)().as_deref() == Some(fid.as_str()) {
                                                app.selected_folder_id.set(None);
                                            }
                                            app.folders_version.set((app.folders_version)() + 1);
                                        });
                                    },
                                }
                            } else {
                                button {
                                    class: kit::MENU_ITEM,
                                    onclick: move |_| {
                                        app.row_menu.set(None);
                                        editing.set(true);
                                    },
                                    IconPencil { size: 16 }
                                    {t(&lang, "folder-menu-rename")}
                                }
                                button {
                                    class: kit::MENU_ITEM,
                                    onclick: move |_| {
                                        app.row_menu.set(None);
                                        creating_sub.set(true);
                                    },
                                    IconFolderPlus { size: 16 }
                                    {t(&lang, "folder-menu-subtheme")}
                                }
                                button {
                                    class: kit::MENU_ITEM,
                                    onclick: move |_| moving.set(true),
                                    IconArrowUpRight { size: 16 }
                                    {t(&lang, "folder-menu-move")}
                                }
                                // Only for a theme that is not already in a space:
                                // sharing takes the whole subtree with it, so it is
                                // offered once and never on a mirrored theme.
                                if right() == FolderRight::Local {
                                    button {
                                        class: kit::MENU_ITEM,
                                        disabled: sharing(),
                                        onclick: move |_| {
                                            app.row_menu.set(None);
                                            share_name.set(String::new());
                                            share_collab.set(owned_spaces().is_empty());
                                            share_open.set(true);
                                        },
                                        IconUsersThree { size: 16 }
                                        {t(&lang, "folder-menu-share")}
                                    }
                                }
                                div { class: kit::MENU_SEP }
                                button {
                                    class: kit::MENU_ITEM_DANGER,
                                    onclick: move |_| confirm_delete.set(true),
                                    IconTrash { size: 16 }
                                    {t(&lang, "folder-menu-delete")}
                                }
                            }
                        }
                    }
                }
            }
            if let Some(e) = share_error() {
                p { class: "text-xs text-ios-red-dark px-2 pb-1 ml-7", "{e}" }
            }
            if share_open() {
                div {
                    class: "bg-stone-100 rounded-xl p-1 ml-7 my-1 flex flex-col",
                    style: "animation: popIn 0.16s ease-out;",
                    for s in owned_spaces() {
                        {
                            let sid = s.id.clone();
                            rsx! {
                                button {
                                    key: "{s.id}",
                                    class: kit::MENU_ITEM,
                                    disabled: sharing(),
                                    onclick: move |_| share(ShareTarget::Existing(sid.clone())),
                                    IconUsersThree { size: 16 }
                                    span { class: "truncate", "{s.name}" }
                                }
                            }
                        }
                    }
                    div { class: "flex items-center gap-1 pl-3 pr-0",
                        input {
                            class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                            placeholder: t(&lang, "space-share-new-team"),
                            value: "{share_name}",
                            maxlength: "{MAX_TEAM_NAME_CHARS}",
                            oninput: move |evt| share_name.set(evt.value()),
                            onkeydown: move |evt| {
                                if evt.key() == Key::Escape {
                                    share_open.set(false);
                                }
                            },
                            onkeypress: move |evt| {
                                if evt.key() == Key::Enter && !share_name().trim().is_empty() {
                                    share(ShareTarget::New(share_name().trim().to_string()));
                                }
                            },
                        }
                        button {
                            class: "shrink-0 w-8 h-7 flex items-center justify-center rounded-md transition-colors duration-150",
                            class: if share_collab() { "bg-ios-orange-50 text-ios-orange-dark" } else { "text-stone-400" },
                            title: t(&lang, "space-mode-collab"),
                            onclick: move |_| share_collab.set(true),
                            IconPencil { size: 14 }
                        }
                        button {
                            class: "shrink-0 w-8 h-7 flex items-center justify-center rounded-md transition-colors duration-150",
                            class: if share_collab() { "text-stone-400" } else { "bg-ios-orange-50 text-ios-orange-dark" },
                            title: t(&lang, "space-mode-read"),
                            onclick: move |_| share_collab.set(false),
                            IconLockSimple { size: 14 }
                        }
                        button {
                            class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                            class: if share_name().trim().is_empty() || sharing() {
                                "text-stone-300"
                            } else {
                                "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                            },
                            onclick: move |_| {
                                if !share_name().trim().is_empty() && !sharing() {
                                    share(ShareTarget::New(share_name().trim().to_string()));
                                }
                            },
                            IconCheck { size: 16 }
                        }
                    }
                }
            }
            if creating_sub() {
                div {
                    class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 ml-7 my-1",
                    style: "animation: popIn 0.16s ease-out;",
                    input {
                        class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                        placeholder: t(&lang, "sidebar-subfolder-placeholder"),
                        value: "{sub_name}",
                        oninput: move |evt| sub_name.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                creating_sub.set(false);
                            }
                        },
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter && !sub_name().trim().is_empty() {
                                let name = sub_name().trim().to_string();
                                sub_name.set(String::new());
                                creating_sub.set(false);
                                create_sub(name, folder_id_for_sub.clone(), space_parent.clone());
                            }
                        },
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                        class: if sub_name().trim().is_empty() {
                            "text-stone-300"
                        } else {
                            "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                        },
                        onclick: move |_| {
                            if !sub_name().trim().is_empty() {
                                let name = sub_name().trim().to_string();
                                sub_name.set(String::new());
                                creating_sub.set(false);
                                create_sub(name, folder_id_for_sub2.clone(), space_parent2.clone());
                            }
                        },
                        IconCheck { size: 16 }
                    }
                }
            }
            if is_expanded {
                for child in children() {
                    FolderItem { key: "{child.id}", folder: child, depth: depth + 1 }
                }
            }
        }
    }
}
