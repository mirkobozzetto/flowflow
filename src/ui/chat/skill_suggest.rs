use crate::application::hermes_message::Attachment;
use crate::application::hermes_skills::{
    category_symbol, named, slash_matches, slash_query, strip_slash,
};
use crate::application::i18n::t;
use crate::ui::chat::hermes_attach::{attach, taken_skills};
use crate::ui::chat::tools_menu::{
    MENU_ICON, MENU_ROW, ROW_SUB, ROW_TITLE, SECTION,
};
use crate::ui::sf_icon::SfIcon;
use crate::ui::AppState;
use dioxus::prelude::*;

fn light(app: AppState, name: &str) {
    attach(
        app,
        vec![Attachment::Skill {
            name: name.to_string(),
        }],
    );
}

/// Above the Hermes field: the "/" matches in a glass panel, or the skills
/// the typed words name as quiet glass pills. Nothing is added until tapped.
#[component]
pub fn SkillSuggest(input: Signal<String>) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let skills = (app.hermes_skills)()
        .and_then(|s| s.ok())
        .unwrap_or_default();
    let text = input();
    if skills.is_empty() {
        return rsx! {};
    }
    if let Some(query) = slash_query(&text) {
        let hits = slash_matches(&skills, &query);
        return rsx! {
            div { class: "glass-panel absolute bottom-full left-0 right-0 mb-2 z-40 max-h-80 overflow-y-auto overscroll-contain p-2 popover-pop",
                p { class: SECTION, {t(&lang, "hermes-attach-skills")} }
                if hits.is_empty() {
                    p { class: "px-3 py-2 text-sm text-stone-500", {t(&lang, "hermes-skills-no-match")} }
                }
                for skill in hits {
                    button {
                        key: "{skill.name}",
                        class: MENU_ROW,
                        // Keeps the field focused (and the keyboard up) while
                        // the row is tapped: refocusing later would strand
                        // the bar (keyboard/inset.rs).
                        onpointerdown: |e| e.prevent_default(),
                        onclick: {
                            let name = skill.name.clone();
                            move |_| {
                                input.set(strip_slash(&input()));
                                light(app, &name);
                            }
                        },
                        span { class: "{MENU_ICON} text-stone-500", SfIcon { name: category_symbol(&skill.category), size: 20 } }
                        span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                            span { class: ROW_TITLE, "{skill.name}" }
                            span { class: "{ROW_SUB} truncate", "{skill.description}" }
                        }
                    }
                }
            }
        };
    }
    let suggested = named(&skills, &text, &taken_skills(app));
    rsx! {
        if !suggested.is_empty() {
            div { class: "flex flex-wrap items-center gap-1.5 px-2 pb-2",
                for skill in suggested {
                    button {
                        key: "{skill.name}",
                        class: "composer-glass pressable inline-flex items-center gap-1.5 pl-2.5 pr-3 py-1.5 rounded-full border border-stone-300/80 text-stone-700 text-xs font-medium",
                        style: "animation: fadeInUp 0.18s ease-out;",
                        onpointerdown: |e| e.prevent_default(),
                        onclick: {
                            let name = skill.name.clone();
                            move |_| light(app, &name)
                        },
                        span { class: "flex text-ios-orange", SfIcon { name: "sparkles", size: 13 } }
                        "{skill.name}"
                    }
                }
            }
        }
    }
}
