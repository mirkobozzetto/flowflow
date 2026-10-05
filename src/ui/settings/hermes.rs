use crate::application::hermes_chat::{self, KEY_SETTING, URL_SETTING};
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::hermes_problem_text;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

// Address and key a scanned QR code brought, shown in the card until the
// person tests them: a link alone never replaces a saved Hermes.
static OFFERED: Mutex<Option<(String, String)>> = Mutex::new(None);

pub(crate) fn offer_link(url: String, key: String) {
    *OFFERED.lock().unwrap_or_else(|e| e.into_inner()) = Some((url, key));
}

#[derive(Clone, PartialEq)]
enum Probe {
    Idle,
    Testing,
    Ok,
    Failed(String),
}

/// Each person links their own Hermes: its address and key, kept on the
/// device, checked in one tap.
#[component]
pub fn HermesSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let offered =
        use_hook(|| OFFERED.lock().unwrap_or_else(|e| e.into_inner()).take());
    let (first_url, first_key) = offered.clone().unwrap_or_else(|| {
        (
            db().get_setting(URL_SETTING).unwrap_or_default(),
            db().get_setting(KEY_SETTING).unwrap_or_default(),
        )
    });
    let mut url = use_signal(|| first_url);
    let mut key = use_signal(|| first_key);
    let mut probe = use_signal(|| Probe::Idle);

    // Saves the fields, then asks Hermes; one sentence says how it went.
    let mut test = move || {
        let (u, k) = (url().trim().to_string(), key().trim().to_string());
        let _ = db().set_setting(URL_SETTING, &u);
        let _ = db().set_setting(KEY_SETTING, &k);
        probe.set(Probe::Testing);
        spawn(async move {
            let lang = (app.current_lang)();
            probe.set(match hermes_chat::check(&u, &k).await {
                Ok(()) => Probe::Ok,
                Err(e) => Probe::Failed(hermes_problem_text(&lang, &e)),
            });
        });
    };

    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center gap-2",
                crate::ui::icons::HermesAgentIcon { size: 24 }
                h2 { class: "text-lg font-semibold text-stone-900", {t(&lang, "hermes-settings-title")} }
            }
            p { class: "text-xs text-stone-500 leading-relaxed", {t(&lang, "hermes-settings-hint")} }
            if offered.is_some() && probe() == Probe::Idle {
                p { class: "text-sm text-stone-700 leading-relaxed", {t(&lang, "hermes-settings-offered")} }
            }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "hermes-settings-url")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "url",
                    autocapitalize: "off",
                    placeholder: "https://…",
                    value: "{url}",
                    oninput: move |evt| {
                        url.set(evt.value());
                        probe.set(Probe::Idle);
                    },
                }
            }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "hermes-settings-key")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    value: "{key}",
                    oninput: move |evt| {
                        key.set(evt.value());
                        probe.set(Probe::Idle);
                    },
                }
            }
            button {
                class: crate::ui::kit::BTN_PRIMARY,
                disabled: probe() == Probe::Testing || url().trim().is_empty() || key().trim().is_empty(),
                onclick: move |_| test(),
                if probe() == Probe::Testing {
                    {t(&lang, "hermes-settings-testing")}
                } else {
                    {t(&lang, "hermes-settings-test")}
                }
            }
            match probe() {
                Probe::Ok => rsx! {
                    p { class: "text-sm text-ios-green", {t(&lang, "hermes-settings-ok")} }
                },
                Probe::Failed(msg) => rsx! {
                    p { class: "text-sm text-ios-red-dark", "{msg}" }
                },
                _ => rsx! {},
            }
        }
    }
}
