//! The notes chat's model and reasoning level, chosen from the "+" menu's
//! first entry and from the line under the empty chat's title. Same menu as
//! Hermes' model menu: the other models, then the other levels.
use crate::application::chat_model::{self, ChatPick, Family};
use crate::application::hermes_chat::model_label;
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::Provider;
use crate::ui::icons::IconCaretRight;
use crate::ui::settings::ai_card::{Brand, BrandMark};
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// Native glass anchor of the empty chat's model line (glass_burger.rs, .ts).
const ANCHOR: &str = "chat-model";

fn conversation(app: AppState) -> Option<String> {
    match (app.view)() {
        View::Chat { conversation_id } => conversation_id,
        _ => None,
    }
}

pub(crate) fn brand(source: Provider) -> Brand {
    match chat_model::family(source) {
        Family::Claude => Brand::Claude,
        Family::OpenAi => Brand::OpenAi,
    }
}

fn level_label(lang: &str, level: &str) -> String {
    t(lang, &format!("hermes-effort-{level}"))
}

/// "Réflexion · Moyenne".
pub(crate) fn level_line(lang: &str, pick: &ChatPick) -> String {
    format!("{} · {}", t(lang, "hermes-effort"), level_label(lang, pick.level))
}

/// The pick the open conversation answers with; re-read after each choice.
pub(crate) fn use_pick() -> ChatPick {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let _ = (app.chat_model_version)();
    chat_model::current(&db(), conversation(app).as_deref())
}

fn pick_model(mut app: AppState, db: &Database, source: Provider, model: &str) {
    let conv = conversation(app);
    match chat_model::choose_model(db, conv.as_deref(), source, model) {
        Ok(_) => app.chat_model_version.set((app.chat_model_version)() + 1),
        Err(error) => eprintln!("[chat] model pick failed: {error}"),
    }
}

fn pick_level(mut app: AppState, db: &Database, level: &str) {
    let conv = conversation(app);
    match chat_model::choose_level(db, conv.as_deref(), level) {
        Ok(_) => app.chat_model_version.set((app.chat_model_version)() + 1),
        Err(error) => eprintln!("[chat] level pick failed: {error}"),
    }
}

fn other_levels(pick: &ChatPick) -> Vec<&'static str> {
    chat_model::models(pick.source)
        .iter()
        .find(|m| m.id == pick.model)
        .map(|m| m.levels.iter().copied().filter(|l| *l != pick.level).collect())
        .unwrap_or_default()
}

fn family_title(lang: &str, family: Family) -> String {
    t(
        lang,
        match family {
            Family::Claude => "settings-provider-claude",
            Family::OpenAi => "settings-provider-openai",
        },
    )
}

/// The menu as the hidden tree glass_burger.ts mirrors: the other models,
/// grouped per provider only when two are connected, then the levels.
#[component]
pub fn NativeModelItems() -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let pick = use_pick();
    let groups = chat_model::options(&db(), &pick);
    let grouped = groups.len() > 1;
    rsx! {
        for (family, source, models) in groups {
            div {
                key: "{source}",
                "data-native-submenu": "",
                "data-native-inline": "",
                "data-native-title": if grouped { family_title(&lang, family) } else { String::new() },
                for model in models {
                    button {
                        key: "{model.id}",
                        "data-native-action": "model|{source}|{model.id}",
                        "data-native-symbol": "",
                        onclick: move |_| pick_model(app, &db(), source, model.id),
                        span { "data-native-icon": "",
                            BrandMark { brand: brand(source), size: 22 }
                        }
                        {model_label(model.id)}
                    }
                }
            }
        }
        div { "data-native-submenu": "", "data-native-title": level_line(&lang, &pick),
            for level in other_levels(&pick) {
                button {
                    key: "{level}",
                    "data-native-action": "level|{level}",
                    "data-native-symbol": "",
                    onclick: move |_| pick_level(app, &db(), level),
                    {level_label(&lang, level)}
                }
            }
        }
    }
}

/// The same menu as rows, for the web "+" popover (Mac, older iOS).
#[component]
pub fn WebModelItems() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let pick = use_pick();
    let groups = chat_model::options(&db(), &pick);
    let grouped = groups.len() > 1;
    use super::tools_menu::{MENU_ICON, MENU_ROW, ROW_TITLE, SECTION, SEP};
    rsx! {
        for (family, source, models) in groups {
            if grouped {
                p { key: "{source}-title", class: SECTION, {family_title(&lang, family)} }
            }
            for model in models {
                button {
                    key: "{model.id}",
                    class: MENU_ROW,
                    onclick: move |_| {
                        pick_model(app, &db(), source, model.id);
                        app.show_tools_menu.set(false);
                    },
                    span { class: MENU_ICON, BrandMark { brand: brand(source), size: 20 } }
                    span { class: ROW_TITLE, {model_label(model.id)} }
                }
            }
        }
        div { class: SEP }
        p { class: SECTION, {level_line(&lang, &pick)} }
        for level in other_levels(&pick) {
            button {
                key: "{level}",
                class: MENU_ROW,
                onclick: move |_| {
                    pick_level(app, &db(), level);
                    app.show_tools_menu.set(false);
                },
                span { class: ROW_TITLE, {level_label(&lang, level)} }
            }
        }
    }
}

/// Under the empty chat's title: the model it will answer with. Native iOS
/// opens the menu over it; elsewhere it opens the "+" menu.
#[component]
pub fn ModelLine() -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let pick = use_pick();
    let native = crate::ui::app::native_glass();
    let summary = format!(
        "{} · {}",
        model_label(pick.model),
        level_label(&lang, pick.level)
    );
    rsx! {
        button {
            class: "inline-flex items-center gap-1.5 text-sm text-stone-500 active:opacity-70",
            "data-glass": native.then_some(ANCHOR),
            "aria-label": "{summary}",
            onclick: move |_| app.show_tools_menu.set(true),
            BrandMark { brand: brand(pick.source), size: 14 }
            "{summary}"
            span { class: "inline-flex rotate-90", IconCaretRight { size: 10 } }
        }
        if native {
            div { hidden: true, "data-native-menu": ANCHOR, "data-native-context": "chat-model",
                NativeModelItems {}
            }
        }
    }
}
