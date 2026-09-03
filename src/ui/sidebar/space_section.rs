// One shared space in the sidebar: its header (name, members, invite, menu),
// its themes, and the panels that used to live three screens away in
// Settings: invite, members, new theme, leave or stop sharing.

use crate::application::i18n::t;
use crate::application::space::{self, Departure, SpaceError};
use crate::domain::space::Space;
use crate::infrastructure::backend::spaces::{AgentView, MemberResp};
use crate::infrastructure::persistence::Database;
use crate::ui::clipboard::copy_text;
use crate::ui::icons::*;
use crate::ui::kit;
use crate::ui::{AppState, RowMenu};
use dioxus::prelude::*;
use std::sync::Arc;

use super::folders::FolderItem;

// Which inline panel is open under the header. One at a time: the sidebar is
// narrow, and two open panels would push the themes off screen.
#[derive(Clone, Copy, PartialEq)]
enum Panel {
    None,
    Invite,
    Members,
    Agent,
    NewTheme,
    Rename,
    Leave,
    Stop,
}

#[component]
pub fn SpaceSection(space: Space) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut panel = use_signal(|| Panel::None);
    let mut invite_link = use_signal(|| None::<String>);
    let mut status = use_signal(|| None::<String>);
    let mut theme_name = use_signal(String::new);
    let mut theme_collab = use_signal(|| true);
    let mut new_name = use_signal(|| space.name.clone());
    let mut error = use_signal(|| None::<String>);
    let mut members_version = use_signal(|| 0u32);
    let mut agent_name = use_signal(|| "Hermes".to_string());
    let mut agent_write = use_signal(|| true);
    let mut agent_token = use_signal(|| None::<String>);
    let mut agent_version = use_signal(|| 0u32);

    let space_id = space.id.clone();
    let is_owner = space.is_owner;
    let menu_open = (app.row_menu)() == Some(RowMenu::Space(space.id.clone()));

    let folders = use_memo({
        let space_id = space_id.clone();
        move || {
            let _v = (app.folders_version)();
            db().list_root_folders()
                .unwrap_or_default()
                .into_iter()
                .filter(|f| f.space_id.as_deref() == Some(space_id.as_str()))
                .collect::<Vec<_>>()
        }
    });

    // Members are fetched once per space, again after an invite or a removal.
    let members = use_resource({
        let space_id = space_id.clone();
        move || {
            let _v = members_version();
            let space_id = space_id.clone();
            async move { space::members(&db(), &space_id).await.ok() }
        }
    });
    let member_list: Vec<MemberResp> = members().flatten().unwrap_or_default();
    let member_count = member_list.len();

    let agents = use_resource({
        let space_id = space_id.clone();
        move || {
            let _v = agent_version();
            let space_id = space_id.clone();
            async move {
                if !is_owner {
                    return None;
                }
                match space::agents(&db(), &space_id).await {
                    Ok(agents) => Some(agents),
                    Err(e) => {
                        eprintln!("[space] {e}");
                        error.set(Some(t(
                            &(app.current_lang)(),
                            space::error_key(&e),
                        )));
                        None
                    }
                }
            }
        }
    });
    let agent_list: Vec<AgentView> = agents()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .filter(|agent| agent.revoked_at.is_none())
        .collect();

    let mut bump = move || {
        app.folders_version.set((app.folders_version)() + 1);
        app.notes_version.set((app.notes_version)() + 1);
    };
    // Any panel change or menu opening ends the previous gesture: its error
    // and its "link copied" line go with it.
    let mut open = move |p: Panel| {
        panel.set(p);
        status.set(None);
        agent_token.set(None);
        if p == Panel::Agent {
            agent_version.set(agent_version() + 1);
        }
        error.set(None);
    };
    let mut show = move |e: SpaceError| {
        eprintln!("[space] {e}");
        error.set(Some(t(&(app.current_lang)(), space::error_key(&e))))
    };

    let id_invite = space_id.clone();
    let id_menu = space_id.clone();
    let id_theme = space_id.clone();
    let id_rename = space_id.clone();
    let id_stop = space_id.clone();
    let id_keep = space_id.clone();
    let id_withdraw = space_id.clone();
    let id_remove = space_id.clone();
    let id_agent_create = space_id.clone();

    let open_invite = use_callback(move |_: ()| {
        open(Panel::Invite);
        let id = id_invite.clone();
        spawn(async move {
            match space::invite_link(&db(), &id).await {
                Ok(link) => invite_link.set(Some(link)),
                Err(e) => {
                    invite_link.set(None);
                    show(e);
                }
            }
        });
    });

    let create_theme = use_callback(move |_: ()| {
        let name = theme_name().trim().to_string();
        if name.is_empty() {
            return;
        }
        let id = id_theme.clone();
        let collab = theme_collab();
        theme_name.set(String::new());
        open(Panel::None);
        spawn(async move {
            match space::create_folder(&db(), &id, None, &name, collab).await {
                Ok(_) => bump(),
                Err(e) => show(e),
            }
        });
    });

    let create_space_agent = use_callback(move |_: ()| {
        let name = agent_name().trim().to_string();
        if name.is_empty() {
            return;
        }
        let id = id_agent_create.clone();
        let scope = if agent_write() { "read_write" } else { "read" };
        spawn(async move {
            match space::create_agent(&db(), &id, &name, scope).await {
                Ok(created) => {
                    agent_name.set("Hermes".to_string());
                    agent_token.set(Some(created.token));
                    agent_version.set(agent_version() + 1);
                    members_version.set(members_version() + 1);
                }
                Err(e) => show(e),
            }
        });
    });

    rsx! {
        div { class: "relative",
            div { class: "flex items-center justify-between pl-2 mb-1",
                span { class: "flex items-center gap-1.5 min-w-0 text-sm font-semibold text-stone-700",
                    span { class: "truncate", "{space.name}" }
                    if member_count > 0 {
                        span { class: "text-xs font-normal text-stone-400", "· {member_count}" }
                    }
                }
                div { class: "flex items-center shrink-0",
                    if is_owner {
                        button {
                            class: "w-11 h-11 flex items-center justify-center rounded-full transition-all duration-200",
                            class: if panel() == Panel::Invite {
                                "rotate-45 text-ios-orange-dark bg-ios-orange-50"
                            } else {
                                "text-ios-orange-dark hover:bg-ios-orange-50"
                            },
                            onclick: move |_| {
                                app.row_menu.set(None);
                                if panel() == Panel::Invite {
                                    open(Panel::None);
                                } else {
                                    open_invite(());
                                }
                            },
                            IconPlus { size: 18 }
                        }
                    }
                    button {
                        class: "w-9 h-11 flex items-center justify-center transition-colors duration-150",
                        class: if menu_open { "text-stone-600" } else { "text-stone-400 hover:text-stone-600" },
                        onclick: move |evt| {
                            evt.stop_propagation();
                            status.set(None);
                            app.row_menu.set(if menu_open {
                                None
                            } else {
                                Some(RowMenu::Space(id_menu.clone()))
                            });
                        },
                        IconDotsThree { size: 20 }
                    }
                }
            }
            if menu_open {
                div {
                        id: "row-menu",
                        class: "absolute right-2 top-full {kit::MENU_PANEL}",
                        onclick: move |evt| evt.stop_propagation(),
                    if is_owner {
                        div { class: "px-3 pt-2 pb-1 text-[11px] text-stone-400",
                            {t(&lang, "space-menu-owner")}
                        }
                        button {
                            class: kit::MENU_ITEM,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                open_invite(());
                            },
                            IconLink { size: 16 }
                            {t(&lang, "space-menu-invite")}
                        }
                        button {
                            class: kit::MENU_ITEM,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                open(Panel::Agent);
                            },
                            HermesAgentIcon { size: 16 }
                            {t(&lang, "space-menu-agent")}
                        }
                    }
                    button {
                        class: kit::MENU_ITEM,
                        onclick: move |_| {
                            app.row_menu.set(None);
                            open(Panel::Members);
                        },
                        IconUsersThree { size: 16 }
                        {t(&lang, "space-menu-members")}
                        if member_count > 0 {
                            span { class: "ml-auto text-xs text-stone-400", "{member_count}" }
                        }
                    }
                    if is_owner {
                        button {
                            class: kit::MENU_ITEM,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                new_name.set(space.name.clone());
                                open(Panel::Rename);
                            },
                            IconPencil { size: 16 }
                            {t(&lang, "space-menu-rename")}
                        }
                        button {
                            class: kit::MENU_ITEM,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                open(Panel::NewTheme);
                            },
                            IconFolderPlus { size: 16 }
                            {t(&lang, "space-menu-new-theme")}
                        }
                        div { class: kit::MENU_SEP }
                        button {
                            class: kit::MENU_ITEM_DANGER,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                open(Panel::Stop);
                            },
                            IconLockSimple { size: 16 }
                            {t(&lang, "space-menu-stop")}
                        }
                    } else {
                        div { class: kit::MENU_SEP }
                        button {
                            class: kit::MENU_ITEM_DANGER,
                            onclick: move |_| {
                                app.row_menu.set(None);
                                open(Panel::Leave);
                            },
                            IconArrowUpRight { size: 16 }
                            {t(&lang, "space-menu-leave")}
                        }
                    }
                }
            }
        }

        if let Some(e) = error() {
            p { class: "text-xs text-ios-red-dark px-2 pb-1", "{e}" }
        }
        if let Some(s) = status() {
            p { class: "text-xs text-stone-500 px-2 pb-1", "{s}" }
        }

        match panel() {
            Panel::Invite => rsx! {
                div { class: "bg-stone-100 rounded-xl p-2 ml-7 my-1 flex flex-col gap-1",
                    button {
                        class: kit::MENU_ITEM,
                        disabled: invite_link().is_none(),
                        onclick: move |_| {
                            if let Some(link) = invite_link() {
                                copy_text(&link);
                                open(Panel::None);
                                status.set(Some(t(&(app.current_lang)(), "space-invite-link-copied")));
                            }
                        },
                        IconLink { size: 16 }
                        {t(&lang, "space-invite-copy")}
                    }
                    button {
                        class: kit::MENU_ITEM,
                        disabled: invite_link().is_none(),
                        onclick: move |_| {
                            if let Some(link) = invite_link() {
                                send_link(&link);
                                open(Panel::None);
                            }
                        },
                        IconExport { size: 16 }
                        {t(&lang, "space-invite-send")}
                    }
                    p { class: "px-3 pb-1 text-[11px] text-stone-400", {t(&lang, "space-invite-validity")} }
                }
            },
            Panel::Members => rsx! {
                div { class: "bg-stone-100 rounded-xl p-2 ml-7 my-1 flex flex-col",
                    if member_list.iter().all(|m| m.me) {
                        p { class: "px-2 py-2 text-xs text-stone-400", {t(&lang, "space-members-alone")} }
                        if is_owner {
                            button {
                                class: kit::MENU_ITEM,
                                onclick: move |_| open_invite(()),
                                IconLink { size: 16 }
                                {t(&lang, "space-menu-invite")}
                            }
                        }
                    }
                    for m in member_list.clone() {
                        MemberRow {
                            key: "{m.web_user_id}",
                            member: m.clone(),
                            can_remove: is_owner && !m.is_owner,
                            on_remove: {
                                let id = id_remove.clone();
                                let who = m.web_user_id.clone();
                                move |_| {
                                    let id = id.clone();
                                    let who = who.clone();
                                    spawn(async move {
                                        match space::remove_member(&db(), &id, &who).await {
                                            Ok(()) => members_version.set(members_version() + 1),
                                            Err(e) => show(e),
                                        }
                                    });
                                }
                            },
                        }
                    }
                }
            },
            Panel::Agent => rsx! {
                div {
                    class: "bg-stone-100 rounded-xl p-2 ml-7 my-1 flex flex-col gap-2",
                    style: "animation: popIn 0.16s ease-out;",
                    div { class: "px-1 flex items-center gap-2 text-sm font-medium text-stone-900",
                        HermesAgentIcon { size: 24 }
                        {t(&lang, "space-agent-title")}
                    }
                    p { class: "px-1 text-xs text-stone-500",
                        {t(&lang, "space-agent-hint")}
                    }
                    if let Some(token) = agent_token() {
                        div { class: "rounded-lg bg-warm-white p-2 flex flex-col gap-2",
                            code {
                                class: "text-[10px] leading-4 break-all select-all text-stone-700",
                                "{token}"
                            }
                            button {
                                class: kit::MENU_ITEM,
                                onclick: {
                                    let token = token.clone();
                                    move |_| {
                                        copy_text(&token);
                                        status.set(Some(t(
                                            &(app.current_lang)(),
                                            "space-agent-token-copied",
                                        )));
                                    }
                                },
                                IconCopy { size: 16 }
                                {t(&lang, "space-agent-copy-token")}
                            }
                        }
                        p { class: "px-1 text-[11px] text-ios-red-dark",
                            {t(&lang, "space-agent-token-once")}
                        }
                    }
                    for agent in agent_list.clone() {
                        AgentRow {
                            key: "{agent.agent_id}",
                            agent: agent.clone(),
                            on_rotate: {
                                let id = space_id.clone();
                                let agent_id = agent.agent_id.clone();
                                let scope = agent.scope.clone();
                                move |_| {
                                    let id = id.clone();
                                    let agent_id = agent_id.clone();
                                    let scope = scope.clone();
                                    spawn(async move {
                                        match space::rotate_agent_token(
                                            &db(),
                                            &id,
                                            &agent_id,
                                            &scope,
                                        )
                                        .await
                                        {
                                            Ok(rotated) => {
                                                agent_token.set(Some(rotated.token));
                                                agent_version.set(agent_version() + 1);
                                            }
                                            Err(e) => show(e),
                                        }
                                    });
                                }
                            },
                            on_revoke: {
                                let id = space_id.clone();
                                let agent_id = agent.agent_id.clone();
                                move |_| {
                                    let id = id.clone();
                                    let agent_id = agent_id.clone();
                                    spawn(async move {
                                        match space::revoke_agent(&db(), &id, &agent_id).await {
                                            Ok(()) => {
                                                agent_token.set(None);
                                                agent_version.set(agent_version() + 1);
                                                members_version.set(members_version() + 1);
                                            }
                                            Err(e) => show(e),
                                        }
                                    });
                                }
                            },
                        }
                    }
                    div { class: "flex flex-col gap-1.5 pt-1",
                        input {
                            class: "h-9 rounded-lg bg-warm-white px-3 text-sm outline-none text-stone-900 placeholder-stone-400",
                            placeholder: t(&lang, "space-agent-name"),
                            value: "{agent_name}",
                            oninput: move |evt| agent_name.set(evt.value()),
                        }
                        div { class: "grid grid-cols-2 gap-1",
                            button {
                                class: "h-8 rounded-lg text-xs transition-colors duration-150",
                                class: if agent_write() {
                                    "bg-warm-white text-stone-500"
                                } else {
                                    "bg-ios-orange-50 text-ios-orange-dark font-medium"
                                },
                                onclick: move |_| agent_write.set(false),
                                {t(&lang, "space-agent-read")}
                            }
                            button {
                                class: "h-8 rounded-lg text-xs transition-colors duration-150",
                                class: if agent_write() {
                                    "bg-ios-orange-50 text-ios-orange-dark font-medium"
                                } else {
                                    "bg-warm-white text-stone-500"
                                },
                                onclick: move |_| agent_write.set(true),
                                {t(&lang, "space-agent-read-write")}
                            }
                        }
                        button {
                            class: "h-9 rounded-lg bg-ios-orange text-white text-sm font-medium transition-opacity duration-150",
                            class: if agent_name().trim().is_empty() {
                                "opacity-40"
                            } else {
                                "active:opacity-70 hover:opacity-90"
                            },
                            disabled: agent_name().trim().is_empty(),
                            onclick: move |_| create_space_agent(()),
                            {t(&lang, "space-agent-create")}
                        }
                    }
                }
            },
            Panel::NewTheme => rsx! {
                div {
                    class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 ml-7 my-1",
                    style: "animation: popIn 0.16s ease-out;",
                    input {
                        class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                        placeholder: t(&lang, "space-new-theme"),
                        value: "{theme_name}",
                        autofocus: true,
                        oninput: move |evt| theme_name.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                open(Panel::None);
                            }
                        },
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter {
                                create_theme(());
                            }
                        },
                    }
                    button {
                        class: "shrink-0 w-8 h-7 flex items-center justify-center rounded-md transition-colors duration-150",
                        class: if theme_collab() { "bg-ios-orange-50 text-ios-orange-dark" } else { "text-stone-400" },
                        title: t(&lang, "space-mode-collab"),
                        onclick: move |_| theme_collab.set(true),
                        IconPencil { size: 14 }
                    }
                    button {
                        class: "shrink-0 w-8 h-7 flex items-center justify-center rounded-md transition-colors duration-150",
                        class: if theme_collab() { "text-stone-400" } else { "bg-ios-orange-50 text-ios-orange-dark" },
                        title: t(&lang, "space-mode-read"),
                        onclick: move |_| theme_collab.set(false),
                        IconLockSimple { size: 14 }
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                        class: if theme_name().trim().is_empty() {
                            "text-stone-300"
                        } else {
                            "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                        },
                        onclick: move |_| create_theme(()),
                        IconCheck { size: 16 }
                    }
                }
            },
            Panel::Rename => rsx! {
                div {
                    class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 ml-7 my-1",
                    style: "animation: popIn 0.16s ease-out;",
                    input {
                        class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900",
                        value: "{new_name}",
                        autofocus: true,
                        oninput: move |evt| new_name.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                open(Panel::None);
                            }
                        },
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center rounded-lg text-ios-orange-dark bg-ios-orange-50 active:opacity-70",
                        onclick: move |_| {
                            let name = new_name().trim().to_string();
                            if name.is_empty() {
                                return;
                            }
                            let id = id_rename.clone();
                            open(Panel::None);
                            spawn(async move {
                                match space::rename(&db(), &id, &name).await {
                                    Ok(()) => bump(),
                                    Err(e) => show(e),
                                }
                            });
                        },
                        IconCheck { size: 16 }
                    }
                }
            },
            Panel::Leave => rsx! {
                div { class: "bg-stone-100 rounded-xl p-2 ml-7 my-1 flex flex-col gap-1.5",
                    p { class: "px-1 text-sm font-medium text-stone-900", {t(&lang, "space-leave-title")} }
                    p { class: "px-1 pb-1 text-xs text-stone-500", {t(&lang, "space-leave-warning")} }
                    button {
                        class: "h-9 flex items-center justify-center rounded-lg bg-warm-white text-stone-900 text-sm font-medium",
                        onclick: move |_| {
                            let id = id_keep.clone();
                            open(Panel::None);
                            spawn(async move {
                                match space::leave(&db(), &id, Departure::KeepMine).await {
                                    Ok(()) => bump(),
                                    Err(e) => show(e),
                                }
                            });
                        },
                        {t(&lang, "space-leave-keep")}
                    }
                    button {
                        class: "h-9 flex items-center justify-center rounded-lg bg-ios-red text-white text-sm font-medium",
                        onclick: move |_| {
                            let id = id_withdraw.clone();
                            open(Panel::None);
                            spawn(async move {
                                match space::leave(&db(), &id, Departure::WithdrawMine).await {
                                    Ok(()) => bump(),
                                    Err(e) => show(e),
                                }
                            });
                        },
                        {t(&lang, "space-leave-withdraw")}
                    }
                }
            },
            Panel::Stop => rsx! {
                div { class: "bg-stone-100 rounded-xl p-2 ml-7 my-1 flex flex-col gap-1.5",
                    p { class: "px-1 text-sm font-medium text-stone-900", {t(&lang, "space-stop-title")} }
                    p { class: "px-1 pb-1 text-xs text-stone-500", {t(&lang, "space-stop-warning")} }
                    div { class: "flex gap-2",
                        button {
                            class: kit::CONFIRM_BTN_GHOST,
                            onclick: move |_| open(Panel::None),
                            {t(&lang, "chat-menu-cancel")}
                        }
                        button {
                            class: kit::CONFIRM_BTN_DANGER,
                            onclick: move |_| {
                                let id = id_stop.clone();
                                open(Panel::None);
                                spawn(async move {
                                    match space::stop_sharing(&db(), &id).await {
                                        Ok(()) => bump(),
                                        Err(e) => show(e),
                                    }
                                });
                            },
                            {t(&lang, "space-stop-confirm")}
                        }
                    }
                }
            },
            Panel::None => rsx! {},
        }

        for folder in folders() {
            FolderItem { key: "{folder.id}", folder: folder, depth: 0 }
        }
    }
}

#[component]
fn AgentRow(
    agent: AgentView,
    on_rotate: EventHandler<()>,
    on_revoke: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let scope_key = if agent.scope == "read_write" {
        "space-agent-read-write"
    } else {
        "space-agent-read"
    };
    rsx! {
        div { class: "rounded-lg bg-warm-white px-2 py-2 flex flex-col gap-1.5",
            div { class: "flex items-center gap-2",
                span { class: "w-7 h-7 rounded-full bg-ios-orange-50 text-ios-orange-dark flex items-center justify-center",
                    HermesAgentIcon { size: 20 }
                }
                span { class: "flex-1 min-w-0 truncate text-sm text-stone-900",
                    "{agent.name}"
                }
                span { class: "text-[10px] text-stone-400",
                    {t(&lang, scope_key)}
                }
            }
            div { class: "grid grid-cols-2 gap-1",
                button {
                    class: kit::PILL_GHOST,
                    onclick: move |_| on_rotate.call(()),
                    {t(&lang, "space-agent-rotate")}
                }
                button {
                    class: "h-7 rounded-md text-xs text-ios-red-dark hover:bg-ios-red-50 active:opacity-70",
                    onclick: move |_| on_revoke.call(()),
                    {t(&lang, "space-agent-revoke")}
                }
            }
        }
    }
}

#[component]
fn MemberRow(
    member: MemberResp,
    can_remove: bool,
    on_remove: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    // No public name: a short handle keeps two anonymous members apart, so
    // "Remove" still knows who it targets.
    let name = if member.me {
        Some(t(&lang, "space-member-you"))
    } else {
        member.display_name.clone()
    };
    let handle: String = member.author_ref.chars().take(6).collect();
    let initials: String = name
        .as_deref()
        .unwrap_or_default()
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase();
    rsx! {
        div { class: "flex items-center min-h-[44px] px-2 gap-2 text-sm text-stone-900",
            span {
                class: "w-7 h-7 rounded-full text-[11px] font-semibold flex items-center justify-center",
                class: if member.is_owner || member.is_agent {
                    "bg-ios-orange-50 text-ios-orange-dark"
                } else {
                    "bg-warm-white text-stone-600"
                },
                if member.is_agent {
                    HermesAgentIcon { size: 20 }
                } else if name.is_some() {
                    "{initials}"
                } else {
                    IconUserCircle { size: 18 }
                }
            }
            if let Some(n) = name {
                span { class: "flex-1 truncate", "{n}" }
            } else {
                span { class: "flex-1 truncate text-stone-400", "{handle}" }
            }
            if member.is_owner {
                span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-warm-white text-stone-500",
                    {t(&lang, "space-owner")}
                }
            } else if member.is_agent {
                span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-warm-white text-stone-500",
                    {t(&lang, "space-agent-badge")}
                }
            }
            if can_remove {
                button {
                    class: kit::PILL_GHOST,
                    onclick: move |_| on_remove.call(()),
                    {t(&lang, "space-member-remove")}
                }
            }
        }
    }
}

// The iOS share sheet takes the link; elsewhere the clipboard is the only way
// out of the app.
fn send_link(link: &str) {
    #[cfg(target_os = "ios")]
    {
        if let Err(e) = crate::infrastructure::platform::ios::share_text(link) {
            eprintln!("[space] share sheet: {e}");
            copy_text(link);
        }
    }
    #[cfg(not(target_os = "ios"))]
    copy_text(link);
}
