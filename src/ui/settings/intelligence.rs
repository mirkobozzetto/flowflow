use super::ai_card::{Brand, BrandMark, BrandTile};
use crate::application::chat_source;
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::{chatgpt_auth, Provider};
use crate::ui::icons::IconCaretRight;
use crate::ui::kit::{MENU_ITEM, MENU_PANEL, SECTION_LABEL};
use crate::ui::{AppState, SettingsSection, View};
use dioxus::prelude::*;
use std::sync::Arc;

// Native menu anchor of the chat source (glass_burger.rs, glass_burger.ts).
const ANCHOR: &str = "chat-source";
const GROUP: &str = "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden";

fn title(lang: &str, source: Provider) -> String {
    let key = match source {
        Provider::OpenAi => "settings-provider-openai",
        Provider::ChatGpt => "settings-provider-chatgpt",
        _ => "settings-provider-claude",
    };
    t(lang, key)
}

fn kind(lang: &str, source: Provider) -> String {
    if source.is_subscription() {
        t(lang, "settings-source-subscription")
    } else {
        t(lang, "settings-source-api-key")
    }
}

fn page(source: Provider) -> SettingsSection {
    match Brand::of(source) {
        Brand::OpenAi => SettingsSection::AiOpenAi,
        _ => SettingsSection::AiClaude,
    }
}

fn open(mut app: AppState, section: SettingsSection) {
    app.previous_view
        .set(Some(View::SettingsSection(SettingsSection::Intelligence)));
    app.view.set(View::SettingsSection(section));
}

/// "Abonnement connecté · clé configurée": what is set up, in one line.
fn summary(lang: &str, parts: &[(bool, &str)]) -> String {
    let text = parts
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, key)| t(lang, key))
        .collect::<Vec<_>>()
        .join(" · ");
    let text = if text.is_empty() {
        t(lang, "settings-summary-none")
    } else {
        text
    };
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[component]
pub fn IntelligenceSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut max_sources = use_signal(|| {
        db().get_setting("rag_max_sources")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(8)
    });
    let mut debug_trace =
        use_signal(|| db().get_setting("debug_trace").as_deref() == Some("1"));
    let ready = |source| chat_source::is_ready(&db(), source);
    #[cfg(debug_assertions)]
    let claude_subscribed = ready(Provider::ClaudeSubscription);
    #[cfg(not(debug_assertions))]
    let claude_subscribed = false;
    let claude = summary(
        &lang,
        &[
            (claude_subscribed, "settings-summary-subscription"),
            (ready(Provider::Anthropic), "settings-summary-key"),
        ],
    );
    let openai = summary(
        &lang,
        &[
            (chatgpt_auth::is_connected(), "settings-chatgpt-connected"),
            (ready(Provider::OpenAi), "settings-summary-key"),
        ],
    );
    let exa_set = db()
        .get_setting("exa_api_key")
        .is_some_and(|key| !key.trim().is_empty());
    let exa = summary(
        &lang,
        &[
            (exa_set, "settings-summary-key"),
            (true, "settings-web-search"),
        ],
    );

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-chat-role-title")} }
                ChatSourcePicker {}
            }
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-accounts-keys")} }
                div { class: GROUP,
                    ProviderRow { brand: Brand::Claude, name: t(&lang, "settings-provider-claude"), summary: claude, section: SettingsSection::AiClaude }
                    ProviderRow { brand: Brand::OpenAi, name: t(&lang, "settings-provider-openai"), summary: openai, section: SettingsSection::AiOpenAi }
                    ProviderRow { brand: Brand::Exa, name: t(&lang, "settings-provider-exa"), summary: exa, section: SettingsSection::AiExa }
                }
            }
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-search-title")} }
                div { class: GROUP,
                    div { class: "px-4 py-3",
                        div { class: "flex justify-between items-center mb-1",
                            label { class: "text-sm font-medium text-stone-700", {t(&lang, "settings-max-sources")} }
                            span { class: "text-sm font-semibold text-ios-orange", "{max_sources}" }
                        }
                        input {
                            class: "w-full accent-ios-orange",
                            r#type: "range",
                            min: "3",
                            max: "15",
                            value: "{max_sources}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse::<i64>() {
                                    max_sources.set(v);
                                    let _ = db().set_setting("rag_max_sources", &v.to_string());
                                }
                            },
                        }
                        div { class: "flex justify-between text-xs text-stone-400",
                            span { "3" }
                            span { "15" }
                        }
                    }
                    button {
                        class: "w-full px-4 py-3 flex items-center justify-between gap-3 text-left",
                        onclick: move |_| {
                            let on = !debug_trace();
                            debug_trace.set(on);
                            let _ = db().set_setting("debug_trace", if on { "1" } else { "0" });
                        },
                        div {
                            p { class: "text-sm font-medium text-stone-700", {t(&lang, "settings-debug-trace")} }
                            p { class: "text-xs text-stone-400", {t(&lang, "settings-debug-trace-desc")} }
                        }
                        span {
                            class: if debug_trace() {
                                "relative w-11 h-6 shrink-0 rounded-full bg-ios-orange transition-colors duration-200"
                            } else {
                                "relative w-11 h-6 shrink-0 rounded-full bg-stone-300 transition-colors duration-200"
                            },
                            span {
                                class: if debug_trace() {
                                    "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-5"
                                } else {
                                    "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-0"
                                },
                            }
                        }
                    }
                }
            }
            p { class: "text-xs text-stone-400 text-center", {t(&lang, "settings-keys-stored-locally")} }
        }
    }
}

#[component]
fn ProviderRow(
    brand: Brand,
    name: String,
    summary: String,
    section: SettingsSection,
) -> Element {
    let app: AppState = use_context();
    rsx! {
        button {
            class: "pressable w-full min-h-[44px] flex items-center gap-3 px-4 py-3 text-left",
            onclick: move |_| open(app, section),
            BrandTile { brand, small: true }
            span { class: "min-w-0 flex-1",
                span { class: "block text-sm font-medium text-stone-800", "{name}" }
                span { class: "block truncate text-xs text-stone-500", "{summary}" }
            }
            span { class: "shrink-0 text-stone-400", IconCaretRight { size: 14 } }
        }
    }
}

/// What answers the chat, and a menu of the other sources. A source not set
/// up yet opens its page instead.
#[component]
fn ChatSourcePicker() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut active = use_signal(|| chat_source::active(&db()));
    let mut menu_open = use_signal(|| false);
    let native = crate::ui::app::native_glass();
    let current = active();
    let others: Vec<(Provider, bool)> = chat_source::all()
        .into_iter()
        .filter(|&source| source != current)
        .map(|source| (source, chat_source::is_ready(&db(), source)))
        .collect();
    let mut pick = move |source: Provider, ready: bool| {
        menu_open.set(false);
        if ready {
            let _ = chat_source::choose(&db(), source);
            active.set(source);
        } else {
            open(app, page(source));
        }
    };
    let subtitle = |source: Provider, ready: bool| {
        if ready {
            kind(&lang, source)
        } else {
            format!(
                "{} · {}",
                kind(&lang, source),
                t(&lang, "settings-source-not-ready")
            )
        }
    };

    rsx! {
        div { class: "relative",
            button {
                class: "w-full flex items-center gap-3 text-left rounded-xl border border-ios-orange-100 bg-warm-white p-4 shadow-card",
                "data-glass": native.then_some(ANCHOR),
                "aria-label": t(&lang, "settings-reply-with"),
                onclick: move |_| menu_open.set(!menu_open()),
                BrandTile { brand: Brand::of(current), small: false }
                span { class: "min-w-0 flex-1",
                    span { class: "block text-[15px] font-semibold text-stone-900", {title(&lang, current)} }
                    span { class: "block text-xs text-stone-500", {kind(&lang, current)} }
                }
                span { class: "shrink-0 rotate-90 text-stone-400", IconCaretRight { size: 16 } }
            }
            if native {
                div { hidden: true, "data-native-menu": ANCHOR, "data-native-context": "chat-source",
                    for (source, ready) in others.clone() {
                        button {
                            key: "{source}",
                            "data-native-action": "{source}",
                            "data-native-symbol": "",
                            "data-native-subtitle": subtitle(source, ready),
                            onclick: move |_| pick(source, ready),
                            span { "data-native-icon": "",
                                BrandMark { brand: Brand::of(source), size: 22 }
                            }
                            {title(&lang, source)}
                        }
                    }
                }
            } else if menu_open() {
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| menu_open.set(false),
                }
                div { class: format!("{MENU_PANEL} absolute left-0 right-0 top-full mt-2"),
                    p { class: "px-3 pt-2 pb-1 text-xs font-medium text-stone-400", {t(&lang, "settings-reply-with")} }
                    for (source, ready) in others {
                        button {
                            key: "{source}",
                            class: MENU_ITEM,
                            onclick: move |_| pick(source, ready),
                            BrandMark { brand: Brand::of(source), size: 20 }
                            span { class: "min-w-0 flex-1",
                                span { class: "block", {title(&lang, source)} }
                                span { class: "block text-xs text-stone-500", {subtitle(source, ready)} }
                            }
                        }
                    }
                }
            }
        }
    }
}
