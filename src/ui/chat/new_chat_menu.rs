//! "New conversation": with your notes or with Hermes. Shared by the Chats
//! list and the home Chat pill.

use crate::application::i18n::t;
use crate::ui::icons::HermesAgentIcon;
use crate::ui::kit;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;

pub fn open_notes_chat(mut app: AppState) {
    app.sidebar_open.set(false);
    app.sidebar_tab.set(SidebarTab::Chats);
    app.chat_scope.set(None);
    crate::ui::sidebar::navigate_with_slide(
        app,
        View::Chat {
            conversation_id: None,
        },
    );
}

pub fn open_hermes_chat(mut app: AppState) {
    app.sidebar_open.set(false);
    app.sidebar_tab.set(SidebarTab::Chats);
    crate::ui::sidebar::navigate_with_slide(
        app,
        View::HermesChat { session_id: None },
    );
}

/// Hidden source of the native menu on `anchor` (glass_burger.ts reads the
/// labels and icons here; a pick clicks these buttons).
#[component]
pub fn NativeNewChatMenu(anchor: &'static str) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        div {
            hidden: true,
            "data-native-menu": anchor,
            "data-native-context": "new-chat",
            button {
                "data-native-action": "notes",
                "data-native-symbol": "ff.chat.ai",
                onclick: move |_| open_notes_chat(app),
                span { "data-native-icon": "",
                    img { src: asset!("/assets/flowflow-icon-64.png"), alt: "" }
                }
                {t(&lang, "new-chat-with-notes")}
            }
            button {
                "data-native-action": "hermes",
                "data-native-symbol": "person.crop.circle",
                onclick: move |_| open_hermes_chat(app),
                span { "data-native-icon": "",
                    img { src: asset!("/assets/hermes-agent.png"), alt: "" }
                }
                {t(&lang, "new-chat-with-hermes")}
            }
        }
    }
}

/// The same two choices as a web menu, where no native menu shows; the
/// caller places the panel and closes it on `on_pick`.
#[component]
pub fn NewChatChoices(on_pick: EventHandler) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        button {
            class: kit::MENU_ITEM,
            onclick: move |_| {
                on_pick.call(());
                open_notes_chat(app);
            },
            img {
                src: asset!("/assets/flowflow-icon-64.png"),
                class: "w-6 h-6 shrink-0 object-contain",
                alt: "",
            }
            {t(&lang, "new-chat-with-notes")}
        }
        button {
            class: kit::MENU_ITEM,
            onclick: move |_| {
                on_pick.call(());
                open_hermes_chat(app);
            },
            HermesAgentIcon { size: 24 }
            {t(&lang, "new-chat-with-hermes")}
        }
    }
}
