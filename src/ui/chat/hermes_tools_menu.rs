use crate::application::hermes_message::Attachment;
use crate::application::i18n::{t, t_args};
use crate::infrastructure::platform::HAS_CAMERA;
use crate::ui::chat::hermes_attach::{self, use_skills, SkillGroups};
use crate::ui::chat::tools_menu::{
    NativeIcon, MENU_ICON, MENU_ROW, ROW_SUB, ROW_TITLE, SECTION, SEP,
};
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;

/// "99 installed", or why the list is not there.
pub(crate) fn skills_summary(
    lang: &str,
    skills: Option<&SkillGroups>,
) -> String {
    match skills {
        None => t(lang, "hermes-skills-loading"),
        Some(Err(_)) => t(lang, "hermes-skills-unavailable"),
        Some(Ok(groups)) => {
            let count: usize = groups.iter().map(|(_, s)| s.len()).sum();
            t_args(
                lang,
                "hermes-skills-count",
                &[("count", &count.to_string())],
            )
        }
    }
}

pub(crate) fn category_label(lang: &str, category: &str) -> String {
    if category.trim().is_empty() {
        t(lang, "hermes-skills-other")
    } else {
        category.to_string()
    }
}

pub(crate) fn skill_attached(app: AppState, name: &str) -> bool {
    app.hermes_attachments
        .read()
        .iter()
        .any(|a| matches!(a, Attachment::Skill { name: n } if n == name))
}

/// The Hermes "+" as a web popover: Mac, and iOS before the native menu.
#[component]
pub fn HermesToolsMenu() -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut in_skills = use_signal(|| false);
    let skills = use_skills();
    let skills = skills.read();
    let summary = skills_summary(&lang, skills.as_ref());

    rsx! {
        div { class: "fixed inset-0 z-30", onclick: move |_| app.show_tools_menu.set(false) }
        div {
            class: "tools-popover absolute bottom-full left-0 mb-2 z-40 bg-warm-white border border-stone-200 rounded-2xl shadow-menu overflow-hidden",
            role: "dialog",
            "aria-label": t(&lang, "chat-tools-tooltip"),
            onmounted: move |_| { document::eval(include_str!("tools_popover.js")); },
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.stop_propagation();
                    app.show_tools_menu.set(false);
                }
            },
            div {
                class: "tools-pane tools-root tools-scroll p-1.5",
                "data-away": in_skills(),
                "aria-hidden": in_skills(),
                "inert": in_skills().then_some(""),
                button { class: MENU_ROW, onclick: move |_| hermes_attach::pick_photos(app),
                    span { class: MENU_ICON, IconImage { size: 22 } }
                    span { class: ROW_TITLE, {t(&lang, "hermes-attach-photos")} }
                }
                if HAS_CAMERA {
                    button { class: MENU_ROW, onclick: move |_| hermes_attach::take_photo(app),
                        span { class: MENU_ICON, IconCamera { size: 22 } }
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-camera")} }
                    }
                }
                button { class: MENU_ROW, onclick: move |_| hermes_attach::pick_file(app),
                    span { class: MENU_ICON, IconFileArrowUp { size: 22 } }
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-file")} }
                        span { class: ROW_SUB, {t(&lang, "hermes-attach-file-hint")} }
                    }
                }
                div { class: SEP }
                button { class: MENU_ROW, "data-pane": "skills", onclick: move |_| in_skills.set(true),
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-skills")} }
                        span { class: ROW_SUB, role: "status", "{summary}" }
                    }
                    span { class: "text-stone-400", IconCaretRight { size: 16 } }
                }
            }
            div {
                class: "tools-pane tools-sub p-1.5",
                "data-in": in_skills(),
                "aria-hidden": !in_skills(),
                "inert": (!in_skills()).then_some(""),
                div { class: "flex items-center gap-1.5 py-1 pr-1.5 pl-0.5",
                    button {
                        class: "w-9 h-9 shrink-0 rounded-full flex items-center justify-center text-stone-600 hover:bg-stone-100 active:bg-stone-100",
                        "aria-label": t(&lang, "shortcut-back"),
                        onclick: move |_| in_skills.set(false),
                        IconArrowLeft { size: 22 }
                    }
                    span { class: "text-sm font-semibold text-stone-800", {t(&lang, "hermes-attach-skills")} }
                }
                div { class: "tools-list tools-scroll overflow-y-auto pr-1.5 pb-1",
                    if let Some(Ok(groups)) = skills.as_ref() {
                        for (category, list) in groups.iter() {
                            p { key: "{category}", class: SECTION, {category_label(&lang, category)} }
                            for skill in list.iter() {
                                button {
                                    key: "{skill.name}",
                                    class: MENU_ROW,
                                    onclick: {
                                        let name = skill.name.clone();
                                        move |_| hermes_attach::toggle_skill(app, &name)
                                    },
                                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0 break-words",
                                        span { class: ROW_TITLE, "{skill.name}" }
                                        span { class: ROW_SUB, "{skill.description}" }
                                    }
                                    if skill_attached(app, &skill.name) {
                                        span { class: "shrink-0 text-ios-orange", IconCheck { size: 16 } }
                                    }
                                }
                            }
                        }
                    } else {
                        p { class: "px-3 py-2 text-sm text-stone-500", role: "status", "{summary}" }
                    }
                }
            }
        }
    }
}

/// The same entries for the native iOS 26 menu (glass_burger.ts mirrors this
/// hidden tree); each category is a titled group of the Skills submenu.
#[component]
pub fn NativeHermesToolsMenu() -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let skills = use_skills();
    let skills = skills.read();
    let summary = skills_summary(&lang, skills.as_ref());

    rsx! {
        div { hidden: true, "data-native-menu": "hermes-plus", "data-native-context": "hermes",
            div { "data-native-submenu": "", "data-native-inline": "",
                button {
                    "data-native-action": "photos",
                    "data-native-symbol": "photo",
                    onclick: move |_| hermes_attach::pick_photos(app),
                    NativeIcon { IconImage { size: 22 } }
                    {t(&lang, "hermes-attach-photos")}
                }
                if HAS_CAMERA {
                    button {
                        "data-native-action": "camera",
                        "data-native-symbol": "camera",
                        onclick: move |_| hermes_attach::take_photo(app),
                        NativeIcon { IconCamera { size: 22 } }
                        {t(&lang, "hermes-attach-camera")}
                    }
                }
                button {
                    "data-native-action": "file",
                    "data-native-symbol": "doc",
                    "data-native-subtitle": t(&lang, "hermes-attach-file-hint"),
                    onclick: move |_| hermes_attach::pick_file(app),
                    NativeIcon { IconFileArrowUp { size: 22 } }
                    {t(&lang, "hermes-attach-file")}
                }
            }
            div { "data-native-submenu": "", "data-native-inline": "",
                div {
                    "data-native-submenu": "",
                    "data-native-title": t(&lang, "hermes-attach-skills"),
                    "data-native-subtitle": "{summary}",
                    "data-native-symbol": "",
                    if let Some(Ok(groups)) = skills.as_ref() {
                        for (category, list) in groups.iter() {
                            div {
                                key: "{category}",
                                "data-native-submenu": "",
                                "data-native-inline": "",
                                "data-native-title": category_label(&lang, category),
                                for skill in list.iter() {
                                    button {
                                        key: "{skill.name}",
                                        "data-native-action": "skill:{skill.name}",
                                        "data-native-symbol": "",
                                        "data-native-subtitle": "{skill.description}",
                                        "data-native-checked": skill_attached(app, &skill.name).to_string(),
                                        onclick: {
                                            let name = skill.name.clone();
                                            move |_| hermes_attach::toggle_skill(app, &name)
                                        },
                                        "{skill.name}"
                                    }
                                }
                            }
                        }
                    } else {
                        button {
                            "data-native-action": "skills-status",
                            "data-native-symbol": "",
                            disabled: true,
                            "{summary}"
                        }
                    }
                }
            }
        }
    }
}
