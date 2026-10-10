use crate::application::i18n::{t, t_args};
use crate::application::push::{self, PushError, TEST_DELAY_SECS};
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const TEST_BUTTON: &str = "w-full min-h-[44px] px-3 rounded-xl border border-stone-200 bg-warm-white flex items-center justify-center gap-2 text-sm font-medium text-stone-600 active:bg-stone-100 transition-colors disabled:opacity-45";

fn problem_text(lang: &str, e: &PushError) -> String {
    t(
        lang,
        match e {
            PushError::Denied => "push-denied",
            PushError::NotPremium => "push-not-premium",
            PushError::NoDevice => "push-no-device",
            PushError::Unavailable => "push-unavailable",
            PushError::NoBackend | PushError::Failed(_) => "push-failed",
        },
    )
}

/// Notifications on this iPhone, under the Hermes link they belong to.
#[component]
pub fn HermesNotifications() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut enabled = use_signal(|| push::is_enabled(&db()));
    let mut busy = use_signal(|| false);
    let mut line = use_signal(|| None::<String>);

    let toggle = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        line.set(None);
        spawn(async move {
            let lang = (app.current_lang)();
            let database = db();
            let turning_on = !enabled();
            let result = if turning_on {
                push::enable(&database).await
            } else {
                push::disable(&database).await
            };
            match result {
                Ok(()) => enabled.set(turning_on),
                Err(e) => line.set(Some(problem_text(&lang, &e))),
            }
            busy.set(false);
        });
    };

    let test = move |_| {
        busy.set(true);
        line.set(None);
        spawn(async move {
            let lang = (app.current_lang)();
            let body = t(&lang, "push-test-body");
            line.set(Some(match push::send_test(&db(), &body).await {
                Ok(()) => t_args(
                    &lang,
                    "push-test-sent",
                    &[("secs", &TEST_DELAY_SECS.to_string())],
                ),
                Err(e) => problem_text(&lang, &e),
            }));
            busy.set(false);
        });
    };

    rsx! {
        div { class: "h-px bg-stone-200" }
        button {
            class: "w-full min-h-[44px] flex items-center justify-between gap-3 text-left",
            disabled: busy(),
            onclick: toggle,
            div {
                p { class: "text-sm font-medium text-stone-700", {t(&lang, "push-settings-title")} }
                p { class: "text-xs text-stone-400", {t(&lang, "push-settings-hint")} }
            }
            span {
                class: if enabled() {
                    "relative w-11 h-6 shrink-0 rounded-full bg-ios-orange transition-colors duration-200"
                } else {
                    "relative w-11 h-6 shrink-0 rounded-full bg-stone-300 transition-colors duration-200"
                },
                span {
                    class: if enabled() {
                        "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-5"
                    } else {
                        "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-0"
                    },
                }
            }
        }
        if enabled() {
            button {
                class: TEST_BUTTON,
                disabled: busy(),
                onclick: test,
                {t(&lang, "push-test")}
            }
        }
        if let Some(text) = line() {
            p { class: "text-xs text-stone-500 leading-relaxed", "{text}" }
        }
    }
}
