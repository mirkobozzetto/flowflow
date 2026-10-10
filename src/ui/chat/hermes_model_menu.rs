use crate::application::hermes_chat::{
    self, effective_effort, efforts_for, menu_providers, model_label,
    provider_label,
};
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{HermesAgentIcon, IconCaretRight, IconCheck};
use crate::ui::kit;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// Native glass anchor of the model menu (glass_burger.rs, glass_burger.ts).
const ANCHOR: &str = "hermes-model";

// The open conversation, or the latest-pick slot before its first message.
fn pick_slot(app: AppState) -> String {
    match (app.view)() {
        View::HermesChat {
            session_id: Some(sid),
        } => sid,
        _ => hermes_chat::LAST_PICK.to_string(),
    }
}

fn pick_model(mut app: AppState, db: &Database, provider: &str, model: &str) {
    app.hermes_pick
        .set(Some((provider.to_string(), model.to_string())));
    // A level the new model does not accept falls back to the default.
    let current = (app.hermes_effort)();
    let kept = effective_effort(current.as_deref(), provider, model);
    app.hermes_effort.set(kept.clone());
    let slot = pick_slot(app);
    let _ = hermes_chat::choose_model(db, &slot, provider, model);
    if let Some(e) = &kept {
        let _ = hermes_chat::choose_effort(db, &slot, e);
    }
}

fn pick_effort(mut app: AppState, db: &Database, effort: &str) {
    app.hermes_effort.set(Some(effort.to_string()));
    let _ = hermes_chat::choose_effort(db, &pick_slot(app), effort);
}

fn effort_label(lang: &str, effort: &str) -> String {
    t(lang, &format!("hermes-effort-{effort}"))
}

/// "Hermes" and, under it, the model and reasoning level this conversation
/// answers with; a tap opens the newest models of the signed-in providers
/// and the levels the model accepts.
#[component]
pub fn HermesModelTitle() -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut open = use_signal(|| false);
    let native = crate::ui::app::native_glass();
    let options = (app.hermes_models)();
    let (provider, model) = match ((app.hermes_pick)(), &options) {
        (Some(p), _) => p,
        (None, Some(o)) => (o.provider.clone(), o.model.clone()),
        (None, None) => (String::new(), String::new()),
    };
    let effort =
        effective_effort((app.hermes_effort)().as_deref(), &provider, &model);
    let levels = efforts_for(&provider, &model);
    let mut others_open = use_signal(|| false);
    let providers = options
        .as_ref()
        .map(|o| menu_providers(o, (&provider, &model)))
        .unwrap_or_default();
    let menu_ready = !providers.is_empty();
    let subtitle = match &effort {
        Some(e) => {
            format!("{} · {}", model_label(&model), effort_label(&lang, e))
        }
        None => model_label(&model),
    };
    let effort_title = match &effort {
        Some(e) => format!(
            "{} · {}",
            t(&lang, "hermes-effort"),
            effort_label(&lang, e)
        ),
        None => t(&lang, "hermes-effort"),
    };

    rsx! {
        div { class: "relative flex-1 min-w-0 flex items-center gap-2",
            HermesAgentIcon { size: 28 }
            div { class: "min-w-0 flex flex-col justify-center",
                span { class: "text-lg font-semibold tracking-[-0.01em] text-stone-900 leading-tight truncate",
                    {t(&lang, "hermes-title")}
                }
                if !model.is_empty() {
                    button {
                        class: "self-start inline-flex items-center gap-1 text-xs leading-tight text-ios-orange-dark active:opacity-70",
                        "data-glass": (native && menu_ready).then_some(ANCHOR),
                        "aria-label": "{subtitle}",
                        disabled: !menu_ready,
                        onclick: move |_| open.set(!open()),
                        "{subtitle}"
                        span { class: "inline-flex rotate-90", IconCaretRight { size: 10 } }
                    }
                }
            }
            if menu_ready && native {
                div { hidden: true, "data-native-menu": ANCHOR, "data-native-context": "hermes",
                    for (i, p) in providers.iter().enumerate() {
                        // The provider in use is listed; the others fold into
                        // their own submenu.
                        div { key: "{p.slug}", "data-native-submenu": "", "data-native-inline": (i == 0).then_some(""),
                            "data-native-title": provider_label(&p.slug, &p.name),
                            for m in p.models.iter() {
                                button {
                                    key: "{m}",
                                    "data-native-action": "{p.slug}|{m}",
                                    "data-native-symbol": "",
                                    "data-native-checked": (p.slug == provider && *m == model).to_string(),
                                    onclick: {
                                        let (slug, m) = (p.slug.clone(), m.clone());
                                        move |_| pick_model(app, &db(), &slug, &m)
                                    },
                                    {model_label(m)}
                                }
                            }
                        }
                    }
                    if !levels.is_empty() {
                        div { "data-native-submenu": "", "data-native-title": "{effort_title}", "data-native-symbol": "brain",
                            for e in levels.iter() {
                                button {
                                    key: "{e}",
                                    "data-native-action": "effort|{e}",
                                    "data-native-symbol": "",
                                    "data-native-checked": (effort.as_deref() == Some(*e)).to_string(),
                                    onclick: move |_| pick_effort(app, &db(), e),
                                    {effort_label(&lang, e)}
                                }
                            }
                        }
                    }
                    button {
                        "data-native-action": "jobs",
                        "data-native-symbol": "clock",
                        onclick: move |_| crate::ui::hermes_jobs::open(app),
                        {t(&lang, "hermes-jobs-title")}
                    }
                }
            }
            if open() && menu_ready {
                div { class: "fixed inset-0 z-40", onclick: move |_| open.set(false) }
                div { class: "absolute left-8 top-full mt-2 max-h-[60vh] overflow-y-auto {kit::MENU_PANEL}",
                    for (i, p) in providers.iter().enumerate() {
                        if i == 0 {
                            p { key: "{p.slug}", class: "px-3 pt-2 pb-1 text-xs font-semibold text-stone-500",
                                {provider_label(&p.slug, &p.name)}
                            }
                        } else {
                            button {
                                key: "{p.slug}",
                                class: kit::MENU_ITEM,
                                onclick: move |_| others_open.set(!others_open()),
                                span { class: "flex-1", {provider_label(&p.slug, &p.name)} }
                                span {
                                    class: if others_open() { "inline-flex text-stone-400 rotate-90 transition-transform duration-180" } else { "inline-flex text-stone-400 transition-transform duration-180" },
                                    IconCaretRight { size: 12 }
                                }
                            }
                        }
                        for m in p.models.iter().filter(|_| i == 0 || others_open()) {
                            button {
                                key: "{p.slug}{m}",
                                class: kit::MENU_ITEM,
                                onclick: {
                                    let (slug, m) = (p.slug.clone(), m.clone());
                                    move |_| {
                                        open.set(false);
                                        pick_model(app, &db(), &slug, &m);
                                    }
                                },
                                span { class: "w-4 shrink-0 text-stone-900",
                                    if p.slug == provider && *m == model {
                                        IconCheck { size: 14 }
                                    }
                                }
                                {model_label(m)}
                            }
                        }
                    }
                    if !levels.is_empty() {
                        p { class: "px-3 pt-2 pb-1 text-xs font-semibold text-stone-500",
                            {t(&lang, "hermes-effort")}
                        }
                        for e in levels.iter() {
                            button {
                                key: "effort-{e}",
                                class: kit::MENU_ITEM,
                                onclick: move |_| {
                                    open.set(false);
                                    pick_effort(app, &db(), e);
                                },
                                span { class: "w-4 shrink-0 text-stone-900",
                                    if effort.as_deref() == Some(*e) {
                                        IconCheck { size: 14 }
                                    }
                                }
                                {effort_label(&lang, e)}
                            }
                        }
                    }
                    div { class: kit::MENU_SEP }
                    button {
                        class: kit::MENU_ITEM,
                        onclick: move |_| {
                            open.set(false);
                            crate::ui::hermes_jobs::open(app);
                        },
                        span { class: "w-4 shrink-0" }
                        {t(&lang, "hermes-jobs-title")}
                    }
                }
            }
        }
    }
}
