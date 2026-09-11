use crate::application::i18n::{t, t_args};
use crate::infrastructure::backend::onboarding::{
    PremiumStatus, RequestStatus,
};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::open_url;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn account_url(lang: &str, path: &str) -> String {
    let prefix = if lang == "fr" { "/fr" } else { "" };
    format!("https://account.flowflow.be{prefix}{path}")
}

#[component]
pub fn AccountOnboarding(
    mut reload: Signal<u32>,
    profile_linked: Signal<Option<bool>>,
) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut code = use_signal(|| None::<(String, String)>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);
    let mut copied = use_signal(|| false);
    let data = use_resource(move || {
        let _ = reload();
        async move {
            let database = db();
            match BackendClient::from_db(&database) {
                Some(client) => client.onboarding(&database).await.ok(),
                None => None,
            }
        }
    });
    use_future(move || async move {
        let mut eval = document::eval(
            r#"
            const refresh = () => {
                if (document.visibilityState === 'visible') dioxus.send(true);
            };
            window.__flowflowAccountRefresh = refresh;
            window.addEventListener('focus', refresh);
            document.addEventListener('visibilitychange', refresh);
        "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            reload += 1;
        }
    });
    use_drop(move || {
        document::eval(
            r#"
            const refresh = window.__flowflowAccountRefresh;
            window.removeEventListener('focus', refresh);
            document.removeEventListener('visibilitychange', refresh);
            delete window.__flowflowAccountRefresh;
        "#,
        );
    });
    let state = data.read().as_ref().and_then(|value| value.clone());
    let valid_code = code().filter(|(_, expires)| {
        chrono::DateTime::parse_from_rfc3339(expires)
            .is_ok_and(|date| date > chrono::Utc::now())
    });
    let link_url = account_url(&lang, "/link");
    let register_url = account_url(&lang, "/register");
    let login_url = account_url(&lang, "/login");
    let portal_url = account_url(&lang, "/");

    rsx! {
        section { class: "bg-warm-white rounded-xl border border-stone-200 p-5 space-y-4",
            if profile_linked() == Some(false) {
                    if let Some((token, expires)) = valid_code {
                        h3 { class: "text-base font-semibold text-stone-800", {t(&lang, "onboarding-code-title")} }
                        p { class: "text-sm text-stone-600 leading-relaxed", {t(&lang, "onboarding-code-instructions")} }
                        div { class: "rounded-lg bg-stone-100 p-3 space-y-2",
                            code { class: "block text-sm text-stone-800 break-all select-all", "{token}" }
                            div { class: "flex flex-wrap items-center justify-between gap-2",
                                p { class: "text-xs text-stone-500",
                                    {t_args(&lang, "account-link-expires", &[("time", &expiry_label(&expires, &lang))])}
                                }
                                button {
                                    class: crate::ui::kit::PILL_GHOST,
                                    onclick: move |_| { crate::ui::clipboard::copy_text(&token); copied.set(true); },
                                    {t(&lang, if copied() { "onboarding-copied" } else { "onboarding-copy-code" })}
                                }
                            }
                        }
                        button {
                            class: crate::ui::kit::BTN_PRIMARY,
                            onclick: move |_| open_url(&link_url),
                            {t(&lang, "onboarding-open-link")}
                        }
                        p { class: "text-xs text-stone-500 leading-relaxed", {t(&lang, "onboarding-link-return")} }
                    } else {
                        h3 { class: "text-base font-semibold text-stone-800", {t(&lang, "onboarding-link-title")} }
                        p { class: "text-sm text-stone-600 leading-relaxed", {t(&lang, "onboarding-account-hint")} }
                        if code().is_some() {
                            p { class: "text-sm text-stone-600", {t(&lang, "onboarding-code-expired")} }
                        }
                        button {
                            class: crate::ui::kit::BTN_PRIMARY,
                            disabled: busy(),
                            onclick: move |_| {
                                if busy() { return; }
                                busy.set(true);
                                error.set(false);
                                spawn(async move {
                                    let database = db();
                                    let result = match BackendClient::from_db(&database) {
                                        Some(client) => client.link_begin(&database).await.ok(),
                                        None => None,
                                    };
                                    error.set(result.is_none());
                                    code.set(result);
                                    copied.set(false);
                                    busy.set(false);
                                });
                            },
                            {t(&lang, if busy() { "account-link-generating" } else { "account-link-button" })}
                        }
                        if error() {
                            p { class: "text-sm text-ios-red", role: "alert", {t(&lang, "onboarding-link-failed")} }
                        }
                        div { class: "border-t border-stone-200 pt-4 space-y-3",
                            p { class: "text-xs text-stone-500", {t(&lang, "onboarding-no-account")} }
                            div { class: "flex flex-wrap gap-2",
                                button { class: crate::ui::kit::PILL_GHOST, onclick: move |_| open_url(&register_url), {t(&lang, "onboarding-register")} }
                                button { class: crate::ui::kit::PILL_GHOST, onclick: move |_| open_url(&login_url), {t(&lang, "onboarding-login")} }
                            }
                        }
                    }
                    details { class: "text-xs text-stone-500 leading-relaxed",
                        summary { class: "cursor-pointer py-2", {t(&lang, "onboarding-invitation-title")} }
                        p { class: "pt-1", {t(&lang, "onboarding-invitation-hint")} }
                    }
            } else if profile_linked() == Some(true) {
                if let Some(value) = state {
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        h3 { class: "text-base font-semibold text-stone-800",
                            {t(&lang, match value.premium_status {
                                PremiumStatus::Active => "onboarding-premium-active",
                                PremiumStatus::Expired => "onboarding-premium-expired",
                                PremiumStatus::Inactive => "onboarding-premium-inactive",
                            })}
                        }
                        span { class: "text-xs text-stone-500", {t(&lang, "onboarding-link-confirmed")} }
                    }
                    if let Some(expires) = value.premium_expires_at {
                        p { class: "text-xs text-stone-500", {t_args(&lang, "onboarding-access-expiry", &[("time", &expiry_label(&expires, &lang))])} }
                    }
                    if !value.premium {
                        p { class: "text-sm text-stone-600 leading-relaxed",
                            {t(&lang, match value.request.as_ref().map(|request| &request.status) {
                                Some(RequestStatus::Pending) => "onboarding-request-pending",
                                Some(RequestStatus::Denied) => "onboarding-request-denied",
                                _ => "onboarding-request-none",
                            })}
                        }
                    }
                    button { class: crate::ui::kit::PILL_GHOST, onclick: move |_| open_url(&portal_url), {t(&lang, "onboarding-manage-access")} }
                } else {
                    p { class: "text-sm text-stone-600", {t(&lang, "onboarding-link-confirmed")} }
                    button { class: crate::ui::kit::PILL_GHOST, onclick: move |_| open_url(&portal_url), {t(&lang, "onboarding-manage-access")} }
                }
            } else if data.read().is_none() {
                p { class: "text-sm text-stone-500", role: "status", {t(&lang, "account-loading")} }
            } else {
                p { class: "text-sm text-stone-600", role: "status", {t(&lang, "onboarding-link-unknown")} }
            }
            button {
                class: "min-h-[44px] text-xs text-stone-500 hover:text-stone-800 underline underline-offset-4",
                onclick: move |_| reload += 1,
                {t(&lang, "onboarding-check-link")}
            }
        }
    }
}

fn expiry_label(iso: &str, lang: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format(if lang == "fr" {
                    "%d/%m/%Y %H:%M"
                } else {
                    "%m/%d/%Y %I:%M %p"
                })
                .to_string()
        })
        .unwrap_or_else(|_| iso.to_string())
}
