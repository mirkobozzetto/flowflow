use crate::application::i18n::{t, t_args};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::haptic;
use crate::ui::icons::*;
use crate::ui::state::{SettingsSection, View};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// The mention palette still uses these; its layout and behavior are unchanged.
pub(crate) const ROW: &str = "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-lg text-left hover:bg-stone-100 active:bg-stone-100 transition-colors duration-150";
pub(crate) const LEAD_ICON: &str = "w-[22px] h-[22px] shrink-0 flex items-center justify-center text-stone-500";
pub(crate) const ROW_TITLE: &str = "text-sm font-medium text-stone-800";
pub(crate) const ROW_SUB: &str = "text-xs text-stone-400";
pub(crate) const SECTION: &str =
    "px-3 py-2 text-xs font-medium text-stone-400 uppercase tracking-[0.08em]";
pub(crate) const SEP: &str = "h-px bg-stone-200 mx-3 my-2";

const MENU_ROW: &str = "w-full flex items-center gap-3 px-3 py-2 min-h-[48px] rounded-lg text-left hover:bg-stone-100 active:bg-stone-100";
const MENU_ICON: &str =
    "w-7 h-7 shrink-0 flex items-center justify-center text-stone-500";
const LEAD_TILE: &str = "w-7 h-7 shrink-0 flex items-center justify-center rounded-full bg-stone-100 overflow-hidden";

#[derive(Clone, Copy, PartialEq)]
enum Pane {
    Agents,
    Connectors,
    Threads,
}

/// The connectors of the linked account, fetched when the menu mounts.
pub(crate) fn use_connections() -> Resource<
    Result<
        Vec<crate::infrastructure::backend::Connector>,
        crate::infrastructure::backend::BackendError,
    >,
> {
    let db: Signal<Arc<Database>> = use_context();
    use_resource(move || async move {
        let database = db();
        match BackendClient::from_db(&database) {
            Some(client) => client.list_connectors(&database).await,
            None => Ok(Vec::new()),
        }
    })
}

/// Opens the chat scoped to the note's folder, coming back to the note.
pub(crate) fn open_note_chat(mut app: AppState, note_id: &str) {
    app.show_note_tools_menu.set(false);
    app.show_folder_picker.set(false);
    app.show_note_menu.set(false);
    app.sidebar_tab.set(crate::ui::SidebarTab::Chats);
    app.chat_scope
        .set((app.detail_folder_id)().map(crate::domain::ChatScope::Folder));
    app.previous_view.set(Some(View::NoteDetail {
        note_id: note_id.to_string(),
    }));
    app.view.set(View::Chat {
        conversation_id: None,
    });
}

/// On a note the agent runs on it; in the chat its command prefills the field.
pub(crate) fn pick_agent(
    mut app: AppState,
    note: bool,
    agent: &crate::application::agent_activation::PaletteAgent,
) {
    if note {
        app.show_note_tools_menu.set(false);
        app.pending_note_agent.set(Some(agent.clone()));
    } else {
        app.show_tools_menu.set(false);
        app.pending_chat_input.set(Some(agent.launch_command()));
    }
}

pub(crate) fn open_connections(mut app: AppState) {
    app.show_tools_menu.set(false);
    app.show_note_tools_menu.set(false);
    app.view
        .set(View::SettingsSection(SettingsSection::Connections));
}

// Both callers already mount this inside the relative wrapper of their + button.
// Keep that desktop shell on iOS too: no viewport sheet or dimming veil.
#[component]
pub fn ToolsMenu(#[props(default = false)] note: bool) -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut close = move || {
        if note {
            app.show_note_tools_menu.set(false);
        } else {
            app.show_tools_menu.set(false);
        }
    };

    rsx! {
        div { class: "fixed inset-0 z-30", onclick: move |_| close() }
        div {
            class: "tools-popover absolute bottom-full left-0 mb-2 z-40 bg-warm-white border border-stone-200 rounded-2xl shadow-menu overflow-hidden",
            role: "dialog",
            "aria-label": t(&lang, "chat-tools-tooltip"),
            onmounted: move |_| { document::eval(include_str!("tools_popover.js")); },
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.stop_propagation();
                    close();
                }
            },
            ToolsMenuBody { note }
        }
    }
}

#[component]
fn ToolsMenuBody(note: bool) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut pane = use_signal(|| Pane::Agents);
    let mut expanded = use_signal(|| false);
    let web_on = (app.chat_web)();
    let has_exa = db()
        .get_setting("exa_api_key")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    let agents = crate::application::agent_activation::palette_entries(&db());
    let _ = (app.notes_version)();
    let note_id = note
        .then(|| (app.current_note_id)())
        .flatten()
        .filter(|id| !id.is_empty());
    let in_thread = note_id
        .as_deref()
        .and_then(|id| db().get_note(id).ok().flatten())
        .and_then(|n| n.thread_id);
    let threads = if note {
        db().list_threads().unwrap_or_default()
    } else {
        Vec::new()
    };
    // Fetch once per opening, shared by the root count and the connector pane.
    let connections = use_connections();
    let result = connections.read();
    let connector_rows = result.as_ref().and_then(|r| r.as_ref().ok());
    let connection_summary = match result.as_ref() {
        None => t(&lang, "chat-tools-connection-loading"),
        Some(Err(_)) => t(&lang, "chat-tools-connection-unavailable"),
        Some(Ok(rows)) => t_args(
            &lang,
            "chat-tools-connectors-count",
            &[
                (
                    "connected",
                    &rows.iter().filter(|c| c.connected).count().to_string(),
                ),
                ("available", &rows.len().to_string()),
            ],
        ),
    };
    let agent_count = t_args(
        &lang,
        "chat-tools-agents-count",
        &[("count", &agents.len().to_string())],
    );
    let mut enter = move |next| {
        pane.set(next);
        expanded.set(true);
        haptic("selection");
    };

    rsx! {
        div {
            class: "tools-pane tools-root tools-scroll p-1.5",
            "data-away": expanded(),
            "aria-hidden": expanded(),
            "inert": expanded().then_some(""),
            if let Some(nid) = note_id.clone() {
                button {
                    class: MENU_ROW,
                    onclick: {
                        let nid = nid.clone();
                        move |_| open_note_chat(app, &nid)
                    },
                    span { class: MENU_ICON, IconChatAi { size: 22 } }
                    span { class: "flex-1 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "note-chat-entry")} }
                    }
                }
                if let Some(tid) = in_thread.clone() {
                    button {
                        class: MENU_ROW,
                        onclick: {
                            let nid = nid.clone();
                            move |_| {
                                app.show_note_tools_menu.set(false);
                                crate::ui::thread::actions::open_thread(app, &nid, &tid);
                            }
                        },
                        span { class: MENU_ICON, IconCardsThree { size: 22 } }
                        span { class: "flex-1 min-w-0",
                            span { class: ROW_TITLE, {t(&lang, "thread-open")} }
                        }
                    }
                } else {
                    button {
                        class: MENU_ROW,
                        onclick: move |_| enter(Pane::Threads),
                        span { class: MENU_ICON, IconCardsThree { size: 22 } }
                        span { class: "flex-1 min-w-0",
                            span { class: ROW_TITLE, {t(&lang, "thread-add-to")} }
                        }
                        span { class: "text-stone-400", IconCaretRight { size: 16 } }
                    }
                }
                div { class: SEP }
            }
            button {
                class: MENU_ROW,
                "data-pane": "agents",
                onclick: move |_| enter(Pane::Agents),
                span { class: MENU_ICON,
                    // Phosphor chats-circle, regular: the final icon in #179.
                    svg { width: "22", height: "22", view_box: "0 0 256 256", fill: "currentColor", "aria-hidden": "true",
                        path { d: "M232.07,186.76a80,80,0,0,0-62.5-114.17A80,80,0,1,0,23.93,138.76l-7.27,24.71a16,16,0,0,0,19.87,19.87l24.71-7.27a80.39,80.39,0,0,0,25.18,7.35,80,80,0,0,0,108.34,40.65l24.71,7.27a16,16,0,0,0,19.87-19.86ZM62,159.5a8.28,8.28,0,0,0-2.26.32L32,168l8.17-27.76a8,8,0,0,0-.63-6,64,64,0,1,1,26.26,26.26A8,8,0,0,0,62,159.5Zm153.79,28.73L224,216l-27.76-8.17a8,8,0,0,0-6,.63,64.05,64.05,0,0,1-85.87-24.88A79.93,79.93,0,0,0,174.7,89.71a64,64,0,0,1,41.75,92.48A8,8,0,0,0,215.82,188.23Z" }
                    }
                }
                span { class: "flex-1 min-w-0",
                    span { class: "block text-sm font-medium text-stone-800", {t(&lang, "chat-tools-section-agents")} }
                    span { class: "block text-xs text-stone-400", "{agent_count}" }
                }
                span { class: "text-stone-400", IconCaretRight { size: 16 } }
            }
            button {
                class: MENU_ROW,
                "data-pane": "connectors",
                onclick: move |_| enter(Pane::Connectors),
                span { class: MENU_ICON,
                    svg { width: "22", height: "22", view_box: "0 0 256 256", fill: "currentColor", "aria-hidden": "true",
                        path { d: "M104 40H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V56a16 16 0 0 0-16-16Zm0 64H56V56h48Zm96-64h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16V56a16 16 0 0 0-16-16Zm0 64h-48V56h48Zm-96 32H56a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16Zm0 64H56v-48h48Zm96-64h-48a16 16 0 0 0-16 16v48a16 16 0 0 0 16 16h48a16 16 0 0 0 16-16v-48a16 16 0 0 0-16-16Zm0 64h-48v-48h48Z" }
                    }
                }
                span { class: "flex-1 min-w-0",
                    span { class: "block text-sm font-medium text-stone-800", {t(&lang, "chat-tools-section-connectors")} }
                    span { class: "block text-xs text-stone-400", role: "status", "{connection_summary}" }
                }
                span { class: "text-stone-400", IconCaretRight { size: 16 } }
            }
            if !note {
                div { class: SEP }
                button {
                    class: MENU_ROW,
                    role: "switch",
                    "aria-checked": web_on,
                    onclick: move |_| {
                        app.chat_web.set(!(app.chat_web)());
                        haptic("selection");
                    },
                    span { class: "w-7 h-7 shrink-0 rounded-full bg-stone-900 flex items-center justify-center",
                        img { src: asset!("/assets/exa-mark.svg"), class: "w-3 h-3.5", alt: "" }
                    }
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "chat-tools-web")} }
                        span { class: ROW_SUB,
                            {t(&lang, if has_exa { "chat-tools-web-desc" } else { "chat-tools-web-needs-key" })}
                        }
                    }
                    Switch { on: web_on }
                }
                // Only chat currently supports @ mentions; do not advertise them on notes.
                div { class: "flex items-center gap-1.5 px-3 pt-2 pb-1 text-xs text-stone-400",
                    span { class: "px-1.5 py-0.5 rounded bg-stone-200 border border-stone-300 text-stone-600 font-mono text-[11px]", "@" }
                    span { {t(&lang, "chat-tools-mention-hint")} }
                }
            }
        }
        div {
            class: "tools-pane tools-sub p-1.5",
            "data-in": expanded(),
            "aria-hidden": !expanded(),
            "inert": (!expanded()).then_some(""),
            div { class: "flex items-center gap-1.5 py-1 pr-1.5 pl-0.5",
                button {
                    class: "w-9 h-9 shrink-0 rounded-full flex items-center justify-center text-stone-600 hover:bg-stone-100 active:bg-stone-100",
                    "aria-label": t(&lang, "shortcut-back"),
                    onclick: move |_| {
                        expanded.set(false);
                        haptic("selection");
                    },
                    IconArrowLeft { size: 22 }
                }
                span { class: "text-sm font-semibold text-stone-800",
                    {t(&lang, match pane() {
                        Pane::Agents => "chat-tools-section-agents",
                        Pane::Connectors => "chat-tools-section-connectors",
                        Pane::Threads => "thread-add-to",
                    })}
                }
            }
            div { class: "tools-list tools-scroll overflow-y-auto pr-1.5 pb-1",
                if pane() == Pane::Threads {
                    if let Some(nid) = note_id.clone() {
                        button {
                            class: MENU_ROW,
                            onclick: {
                                let (nid, lang) = (nid.clone(), lang.clone());
                                move |_| {
                                    app.show_note_tools_menu.set(false);
                                    crate::ui::thread::actions::start_thread_with_note(app, &db(), &lang, &nid);
                                }
                            },
                            span { class: MENU_ICON, IconStackPlus { size: 22 } }
                            span { class: ROW_TITLE, {t(&lang, "thread-start")} }
                        }
                        for thread in threads.iter() {
                            button {
                                key: "{thread.id}",
                                class: MENU_ROW,
                                onclick: {
                                    let (nid, tid) = (nid.clone(), thread.id.clone());
                                    move |_| {
                                        app.show_note_tools_menu.set(false);
                                        crate::ui::thread::actions::add_note_to_thread(app, &db(), &nid, &tid);
                                    }
                                },
                                span { class: MENU_ICON, IconCardsThree { size: 22 } }
                                span { class: ROW_TITLE,
                                    {if thread.title.is_empty() { t(&lang, "thread-untitled") } else { thread.title.clone() }}
                                }
                            }
                        }
                    }
                } else if pane() == Pane::Agents {
                    if agents.is_empty() {
                        p { class: "px-3 py-2 text-sm text-stone-500", {t(&lang, "chat-tools-agents-empty")} }
                    }
                    for agent in agents {
                        button {
                            class: MENU_ROW,
                            key: "{agent.id}",
                            onclick: {
                                let agent = agent.clone();
                                move |_| pick_agent(app, note, &agent)
                            },
                            span { class: MENU_ICON, IconChatAi { size: 22 } }
                            span { class: "flex-1 flex flex-col gap-0.5 min-w-0 break-words",
                                span { class: ROW_TITLE, "{agent.name}" }
                                span { class: ROW_SUB,
                                    if note { "{agent.description}" } else { {agent.launch_command()} }
                                }
                            }
                        }
                    }
                } else {
                    if let Some(rows) = connector_rows {
                        if rows.is_empty() {
                            p { class: "px-3 py-2 text-sm text-stone-500", {t(&lang, "chat-tools-connectors-empty")} }
                        }
                        for connector in rows {
                            button {
                                key: "{connector.provider}",
                                class: MENU_ROW,
                                onclick: move |_| open_connections(app),
                                span { class: LEAD_TILE, ConnectorIcon { provider: connector.provider.clone(), size: 18 } }
                                span { class: "flex-1 flex flex-col gap-0.5 min-w-0 break-words",
                                    span { class: ROW_TITLE, "{connector.name}" }
                                    span { class: ROW_SUB,
                                        {t(&lang, if connector.connected { "connections-connected" } else { "connections-not-connected" })}
                                    }
                                }
                                if connector.connected {
                                    span { class: "shrink-0 text-ios-orange", IconCheck { size: 16 } }
                                }
                            }
                        }
                    } else {
                        p { class: "px-3 py-2 text-sm text-stone-500", role: "status", "{connection_summary}" }
                    }
                    button { class: MENU_ROW, onclick: move |_| open_connections(app),
                        span { class: "text-xs text-stone-500", {t(&lang, "chat-tools-manage-connections")} }
                    }
                }
            }
        }
    }
}

/// The "+" menu as a hidden DOM tree that the native iOS menu mirrors
/// (glass_burger.ts): same entries and actions as `ToolsMenu`, run by a click
/// on these buttons. Groups and submenus follow the `data-native-*` contract.
#[component]
pub fn NativeToolsMenu(note: bool) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let _ = (app.notes_version)();
    let agents = crate::application::agent_activation::palette_entries(&db());
    let connections = use_connections();
    let connectors = connections
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let note_id = note
        .then(|| (app.current_note_id)())
        .flatten()
        .filter(|id| !id.is_empty());
    let in_thread = note_id
        .as_deref()
        .and_then(|id| db().get_note(id).ok().flatten())
        .and_then(|n| n.thread_id);
    let threads = if note {
        db().list_threads().unwrap_or_default()
    } else {
        Vec::new()
    };
    let web_on = (app.chat_web)();
    let has_exa = db()
        .get_setting("exa_api_key")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    let context = note_id.clone().unwrap_or_else(|| "chat".to_string());

    rsx! {
        div {
            hidden: true,
            "data-native-menu": if note { "note-plus" } else { "chat-plus" },
            "data-native-context": "{context}",
            if let Some(nid) = note_id.clone() {
                div { "data-native-submenu": "", "data-native-inline": "",
                    button {
                        "data-native-action": "chat",
                        "data-native-symbol": "ff.chat.ai",
                        onclick: {
                            let nid = nid.clone();
                            move |_| open_note_chat(app, &nid)
                        },
                        {t(&lang, "note-chat-entry")}
                    }
                    if let Some(tid) = in_thread {
                        button {
                            "data-native-action": "thread-open",
                            "data-native-symbol": "rectangle.stack",
                            onclick: {
                                let nid = nid.clone();
                                move |_| crate::ui::thread::actions::open_thread(app, &nid, &tid)
                            },
                            {t(&lang, "thread-open")}
                        }
                    } else {
                        div {
                            "data-native-submenu": "",
                            "data-native-title": t(&lang, "thread-add-to"),
                            "data-native-symbol": "rectangle.stack",
                            button {
                                "data-native-action": "thread-new",
                                "data-native-symbol": "plus.rectangle.on.rectangle",
                                onclick: {
                                    let (nid, lang) = (nid.clone(), lang.clone());
                                    move |_| crate::ui::thread::actions::start_thread_with_note(app, &db(), &lang, &nid)
                                },
                                {t(&lang, "thread-start")}
                            }
                            for thread in threads.iter() {
                                button {
                                    key: "{thread.id}",
                                    "data-native-action": "thread:{thread.id}",
                                    "data-native-symbol": "rectangle.stack",
                                    onclick: {
                                        let (nid, tid) = (nid.clone(), thread.id.clone());
                                        move |_| crate::ui::thread::actions::add_note_to_thread(app, &db(), &nid, &tid)
                                    },
                                    {if thread.title.is_empty() { t(&lang, "thread-untitled") } else { thread.title.clone() }}
                                }
                            }
                        }
                    }
                }
            }
            div { "data-native-submenu": "", "data-native-inline": "",
                div {
                    "data-native-submenu": "",
                    "data-native-title": t(&lang, "chat-tools-section-agents"),
                    "data-native-symbol": "bubble.left.and.bubble.right",
                    if agents.is_empty() {
                        button {
                            "data-native-action": "agents-empty",
                            "data-native-symbol": "",
                            disabled: true,
                            {t(&lang, "chat-tools-agents-empty")}
                        }
                    }
                    for agent in agents {
                        button {
                            key: "{agent.id}",
                            "data-native-action": "agent:{agent.id}",
                            "data-native-symbol": "bubble.left.and.bubble.right",
                            "data-native-title": "{agent.name}",
                            "data-native-subtitle": if note { agent.description.clone() } else { agent.launch_command() },
                            onclick: {
                                let agent = agent.clone();
                                move |_| pick_agent(app, note, &agent)
                            },
                            "{agent.name}"
                        }
                    }
                }
                div {
                    "data-native-submenu": "",
                    "data-native-title": t(&lang, "chat-tools-section-connectors"),
                    "data-native-symbol": "square.grid.2x2",
                    for connector in connectors {
                        button {
                            key: "{connector.provider}",
                            "data-native-action": "connector:{connector.provider}",
                            "data-native-symbol": "",
                            "data-native-checked": connector.connected,
                            "data-native-subtitle": t(&lang, if connector.connected { "connections-connected" } else { "connections-not-connected" }),
                            onclick: move |_| open_connections(app),
                            "{connector.name}"
                        }
                    }
                    button {
                        "data-native-action": "connections",
                        "data-native-symbol": "gearshape",
                        onclick: move |_| open_connections(app),
                        {t(&lang, "chat-tools-manage-connections")}
                    }
                }
            }
            if !note {
                div { "data-native-submenu": "", "data-native-inline": "",
                    button {
                        "data-native-action": "web",
                        "data-native-symbol": "globe",
                        "data-native-checked": web_on,
                        "data-native-subtitle": t(&lang, if has_exa { "chat-tools-web-desc" } else { "chat-tools-web-needs-key" }),
                        onclick: move |_| app.chat_web.set(!(app.chat_web)()),
                        {t(&lang, "chat-tools-web")}
                    }
                }
            }
        }
    }
}

#[component]
fn Switch(on: bool) -> Element {
    rsx! {
        span { class: "relative w-11 h-6 shrink-0 rounded-full bg-stone-300", "aria-hidden": "true",
            span { class: "tools-switch-on absolute inset-0 rounded-full bg-ios-orange", "data-on": on }
            span { class: "tools-switch-knob absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-card", "data-on": on }
        }
    }
}
