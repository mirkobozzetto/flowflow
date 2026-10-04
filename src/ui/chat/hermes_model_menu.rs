use crate::application::hermes_chat::{self, model_label, provider_label};
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

fn pick(mut app: AppState, db: &Database, provider: &str, model: &str) {
    app.hermes_pick
        .set(Some((provider.to_string(), model.to_string())));
    if let View::HermesChat {
        session_id: Some(sid),
    } = (app.view)()
    {
        let _ = hermes_chat::choose_model(db, &sid, provider, model);
    }
}

/// "Hermes" and, under it, the model this conversation answers with; a tap
/// opens the models of the signed-in providers.
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
    let menu_ready = options.as_ref().is_some_and(|o| !o.providers.is_empty());

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
                        "aria-label": model_label(&model),
                        disabled: !menu_ready,
                        onclick: move |_| open.set(!open()),
                        {model_label(&model)}
                        span { class: "inline-flex rotate-90", IconCaretRight { size: 10 } }
                    }
                }
            }
            if let Some(options) = options.filter(|_| menu_ready) {
                if native {
                    div { hidden: true, "data-native-menu": ANCHOR, "data-native-context": "hermes",
                        for p in options.providers.iter() {
                            div { key: "{p.slug}", "data-native-submenu": "", "data-native-inline": "",
                                "data-native-title": provider_label(&p.slug, &p.name),
                                for m in p.models.iter() {
                                    button {
                                        key: "{m}",
                                        "data-native-action": "{p.slug}|{m}",
                                        "data-native-symbol": "",
                                        "data-native-checked": (p.slug == provider && *m == model).to_string(),
                                        onclick: {
                                            let (slug, m) = (p.slug.clone(), m.clone());
                                            move |_| pick(app, &db(), &slug, &m)
                                        },
                                        {model_label(m)}
                                    }
                                }
                            }
                        }
                    }
                }
                if open() {
                    div { class: "fixed inset-0 z-40", onclick: move |_| open.set(false) }
                    div { class: "absolute left-8 top-full mt-2 max-h-[60vh] overflow-y-auto {kit::MENU_PANEL}",
                        for p in options.providers.iter() {
                            p { key: "{p.slug}", class: "px-3 pt-2 pb-1 text-xs font-semibold text-stone-500",
                                {provider_label(&p.slug, &p.name)}
                            }
                            for m in p.models.iter() {
                                button {
                                    key: "{p.slug}{m}",
                                    class: kit::MENU_ITEM,
                                    onclick: {
                                        let (slug, m) = (p.slug.clone(), m.clone());
                                        move |_| {
                                            open.set(false);
                                            pick(app, &db(), &slug, &m);
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
                    }
                }
            }
        }
    }
}
