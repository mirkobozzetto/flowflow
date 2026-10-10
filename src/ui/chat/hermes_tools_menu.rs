use crate::application::hermes_chat::{HermesError, Skill};
use crate::application::hermes_skills::{
    by_category, category_label, category_symbol, most_used,
};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::platform::HAS_CAMERA;
use crate::ui::chat::hermes_attach::{self, taken_skills};
use crate::ui::chat::tools_menu::{
    MENU_ICON, MENU_ROW, ROW_SUB, ROW_TITLE, SECTION, SEP,
};
use crate::ui::hermes_jobs::JobsEntries;
use crate::ui::icons::{IconArrowLeft, IconCaretRight, IconCheck};
use crate::ui::sf_icon::SfIcon;
use crate::ui::AppState;
use dioxus::prelude::*;

/// "92 installed", or why the list is not there.
pub(crate) fn skills_summary(
    lang: &str,
    skills: Option<&Result<Vec<Skill>, HermesError>>,
) -> String {
    match skills {
        None => t(lang, "hermes-skills-loading"),
        Some(Err(_)) => t(lang, "hermes-skills-unavailable"),
        Some(Ok(list)) => t_args(
            lang,
            "hermes-skills-count",
            &[("count", &list.len().to_string())],
        ),
    }
}

fn label(lang: &str, category: &str) -> String {
    if category.trim().is_empty() {
        t(lang, "hermes-skills-other")
    } else {
        category_label(category)
    }
}

fn in_category(lang: &str, count: usize) -> String {
    t_args(
        lang,
        "hermes-skills-in-category",
        &[("count", &count.to_string())],
    )
}

#[component]
fn SkillRow(skill: Skill, taken: bool) -> Element {
    let app: AppState = use_context();
    rsx! {
        button {
            class: MENU_ROW,
            onclick: {
                let name = skill.name.clone();
                move |_| hermes_attach::toggle_skill(app, &name)
            },
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0 break-words",
                span { class: ROW_TITLE, "{skill.name}" }
                span { class: ROW_SUB, "{skill.description}" }
            }
            if taken {
                span { class: "shrink-0 text-ios-orange", IconCheck { size: 16 } }
            }
        }
    }
}

/// The Hermes "+" as a web popover: Mac, and iOS before the native menu.
/// Skills open on the most used, then one row per category.
#[component]
pub fn HermesToolsMenu() -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut in_skills = use_signal(|| false);
    let mut category: Signal<Option<String>> = use_signal(|| None);
    let skills = (app.hermes_skills)();
    let summary = skills_summary(&lang, skills.as_ref());
    let list = skills.and_then(|s| s.ok()).unwrap_or_default();
    let taken = taken_skills(app);
    let title = match category() {
        Some(c) => label(&lang, &c),
        None => t(&lang, "hermes-attach-skills"),
    };

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
                    span { class: MENU_ICON, SfIcon { name: "photo.on.rectangle.angled", size: 22 } }
                    span { class: ROW_TITLE, {t(&lang, "hermes-attach-photos")} }
                }
                if HAS_CAMERA {
                    button { class: MENU_ROW, onclick: move |_| hermes_attach::take_photo(app),
                        span { class: MENU_ICON, SfIcon { name: "camera", size: 22 } }
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-camera")} }
                    }
                }
                button { class: MENU_ROW, onclick: move |_| hermes_attach::pick_file(app),
                    span { class: MENU_ICON, SfIcon { name: "doc", size: 22 } }
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-file")} }
                        span { class: ROW_SUB, {t(&lang, "hermes-attach-file-hint")} }
                    }
                }
                div { class: SEP }
                button { class: MENU_ROW, "data-pane": "skills", onclick: move |_| in_skills.set(true),
                    span { class: "{MENU_ICON} text-ios-orange", SfIcon { name: "sparkles", size: 22 } }
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, {t(&lang, "hermes-attach-skills")} }
                        span { class: ROW_SUB, role: "status", "{summary}" }
                    }
                    span { class: "text-stone-400", IconCaretRight { size: 16 } }
                }
                div { class: SEP }
                JobsEntries {
                    class: MENU_ROW,
                    on_pick: move |_| app.show_tools_menu.set(false),
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
                        onclick: move |_| {
                            if category.peek().is_some() {
                                category.set(None);
                            } else {
                                in_skills.set(false);
                            }
                        },
                        IconArrowLeft { size: 22 }
                    }
                    span { class: "text-sm font-semibold text-stone-800", "{title}" }
                }
                div { class: "tools-list tools-scroll overflow-y-auto pr-1.5 pb-1",
                    if list.is_empty() {
                        p { class: "px-3 py-2 text-sm text-stone-500", role: "status", "{summary}" }
                    } else if let Some(c) = category() {
                        for skill in by_category(&list).into_iter().find(|(k, _)| *k == c).map(|(_, s)| s).unwrap_or_default() {
                            SkillRow { key: "{skill.name}", taken: taken.contains(&skill.name), skill: skill.clone() }
                        }
                    } else {
                        p { class: SECTION, {t(&lang, "hermes-skills-most-used")} }
                        for skill in most_used(&list) {
                            SkillRow { key: "top-{skill.name}", taken: taken.contains(&skill.name), skill: skill.clone() }
                        }
                        div { class: SEP }
                        for (c, group) in by_category(&list) {
                            button {
                                key: "{c}",
                                class: MENU_ROW,
                                onclick: {
                                    let c = c.clone();
                                    move |_| category.set(Some(c.clone()))
                                },
                                span { class: MENU_ICON, SfIcon { name: category_symbol(&c), size: 20 } }
                                span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                                    span { class: ROW_TITLE, {label(&lang, &c)} }
                                    span { class: ROW_SUB, {in_category(&lang, group.len())} }
                                }
                                span { class: "text-stone-400", IconCaretRight { size: 16 } }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The same entries for the native iOS 26 menu (glass_burger.ts mirrors this
/// hidden tree), drawn with SF Symbols: the most used skills as a titled
/// group, then one submenu per category.
#[component]
pub fn NativeHermesToolsMenu() -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let skills = (app.hermes_skills)();
    let summary = skills_summary(&lang, skills.as_ref());
    let list = skills.and_then(|s| s.ok()).unwrap_or_default();
    let taken = taken_skills(app);
    let action = |skill: &Skill, prefix: &str| {
        let name = skill.name.clone();
        rsx! {
            button {
                key: "{prefix}{skill.name}",
                "data-native-action": "skill:{prefix}{skill.name}",
                "data-native-symbol": "",
                "data-native-subtitle": "{skill.description}",
                "data-native-checked": taken.contains(&skill.name).to_string(),
                onclick: move |_| hermes_attach::toggle_skill(app, &name),
                "{skill.name}"
            }
        }
    };

    rsx! {
        div { hidden: true, "data-native-menu": "hermes-plus", "data-native-context": "hermes",
            div { "data-native-submenu": "", "data-native-inline": "",
                button {
                    "data-native-action": "photos",
                    "data-native-symbol": "photo.on.rectangle.angled",
                    onclick: move |_| hermes_attach::pick_photos(app),
                    {t(&lang, "hermes-attach-photos")}
                }
                if HAS_CAMERA {
                    button {
                        "data-native-action": "camera",
                        "data-native-symbol": "camera",
                        onclick: move |_| hermes_attach::take_photo(app),
                        {t(&lang, "hermes-attach-camera")}
                    }
                }
                button {
                    "data-native-action": "file",
                    "data-native-symbol": "doc",
                    "data-native-subtitle": t(&lang, "hermes-attach-file-hint"),
                    onclick: move |_| hermes_attach::pick_file(app),
                    {t(&lang, "hermes-attach-file")}
                }
            }
            div { "data-native-submenu": "", "data-native-inline": "",
                div {
                    "data-native-submenu": "",
                    "data-native-title": t(&lang, "hermes-attach-skills"),
                    "data-native-subtitle": "{summary}",
                    "data-native-symbol": "sparkles",
                    if list.is_empty() {
                        button {
                            "data-native-action": "skills-status",
                            "data-native-symbol": "",
                            disabled: true,
                            "{summary}"
                        }
                    } else {
                        div {
                            "data-native-submenu": "",
                            "data-native-inline": "",
                            "data-native-title": t(&lang, "hermes-skills-most-used"),
                            for skill in most_used(&list) {
                                {action(&skill, "top:")}
                            }
                        }
                        for (c, group) in by_category(&list) {
                            div {
                                key: "{c}",
                                "data-native-submenu": "",
                                "data-native-title": label(&lang, &c),
                                "data-native-subtitle": in_category(&lang, group.len()),
                                "data-native-symbol": category_symbol(&c),
                                for skill in group.iter() {
                                    {action(skill, "")}
                                }
                            }
                        }
                    }
                }
            }
            div { "data-native-submenu": "", "data-native-inline": "",
                JobsEntries { class: "", on_pick: move |_| {} }
            }
        }
    }
}
