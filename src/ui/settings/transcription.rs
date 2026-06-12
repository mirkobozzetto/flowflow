use crate::db::settings_repo::{STT_PROVIDER_KEY, WHISPER_MODEL_KEY};
use crate::db::Database;
use crate::services::i18n::{t, t_args};
use crate::services::transcription::models;
use crate::services::transcription::SttProvider;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::str::FromStr;
use std::sync::Arc;

fn size_mb(bytes: u64) -> String {
    format!("{} MB", bytes / 1_048_576)
}

#[component]
pub fn TranscriptionSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut provider = use_signal(|| {
        db().get_setting(STT_PROVIDER_KEY)
            .and_then(|v| SttProvider::from_str(&v).ok())
            .unwrap_or_default()
    });
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            h2 { class: "text-lg font-semibold text-stone-900",
                {t(&lang, "settings-stt-provider-title")}
            }
            div { class: "flex gap-2",
                button {
                    class: if provider() == SttProvider::Soniox {
                        "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-ios-orange text-white"
                    } else {
                        "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-stone-200 text-stone-700"
                    },
                    onclick: move |_| {
                        provider.set(SttProvider::Soniox);
                        let _ = db().set_setting(
                            STT_PROVIDER_KEY,
                            SttProvider::Soniox.as_str(),
                        );
                    },
                    {t(&lang, "settings-stt-cloud")}
                }
                button {
                    class: if provider() == SttProvider::WhisperLocal {
                        "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-ios-orange text-white"
                    } else {
                        "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-stone-200 text-stone-700"
                    },
                    onclick: move |_| {
                        provider.set(SttProvider::WhisperLocal);
                        let _ = db().set_setting(
                            STT_PROVIDER_KEY,
                            SttProvider::WhisperLocal.as_str(),
                        );
                    },
                    {t(&lang, "settings-stt-local")}
                }
            }

            if provider() == SttProvider::Soniox {
                SonioxKeyForm {}
            } else {
                WhisperModelsSection {}
            }
        }
    }
}

#[component]
fn SonioxKeyForm() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut soniox_key =
        use_signal(|| db().get_setting("soniox_api_key").unwrap_or_default());
    let mut saved = use_signal(|| false);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6",
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-soniox-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: t(&lang, "settings-soniox-placeholder"),
                    value: "{soniox_key}",
                    oninput: move |evt| {
                        soniox_key.set(evt.value());
                        saved.set(false);
                    },
                }
            }

            button {
                class: if saved() { crate::ui::kit::BTN_SUCCESS } else { crate::ui::kit::BTN_PRIMARY },
                onclick: move |_| {
                    let sk = soniox_key().trim().to_string();
                    if !sk.is_empty() {
                        let _ = db().set_setting("soniox_api_key", &sk);
                    }
                    saved.set(true);
                },
                if saved() {
                    {t(&lang, "settings-saved")}
                } else {
                    {t(&lang, "settings-save")}
                }
            }

            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "settings-keys-stored-locally")}
            }
        }
    }
}

#[component]
fn WhisperModelsSection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut models_version = use_signal(|| 0u32);
    let mut downloading: Signal<Option<(String, u64, u64)>> =
        use_signal(models::download_progress);
    let mut dl_error: Signal<Option<String>> = use_signal(|| None);
    let mut confirm_dl: Signal<Option<&'static str>> = use_signal(|| None);
    let lang = (app.current_lang)();

    use_future(move || async move {
        if let Some(e) = models::take_download_error() {
            dl_error.set(Some(e));
        }
        let mut last = models::download_progress();
        loop {
            let now = models::download_progress();
            if now != last {
                if now.is_none() {
                    if let Some(e) = models::take_download_error() {
                        dl_error.set(Some(e));
                    }
                    models_version.set(models_version() + 1);
                }
                downloading.set(now.clone());
                last = now;
            }
            futures_timer::Delay::new(std::time::Duration::from_millis(300))
                .await;
        }
    });

    let _ = models_version();
    let dir = models::models_dir();
    let active_model = db().get_setting(WHISPER_MODEL_KEY).unwrap_or_default();

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-base font-semibold text-stone-900",
                {t(&lang, "whisper-models-title")}
            }
            p { class: "text-xs text-stone-500",
                {t(&lang, "whisper-no-model-hint")}
            }
            if let Some(ref err) = dl_error() {
                div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                    p { class: "text-xs text-stone-600 break-words", "{err}" }
                }
            }
            div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                for entry in models::catalog() {
                    {
                        let id = entry.id;
                        let downloaded = models::is_downloaded(&dir, id);
                        let is_active = downloaded && active_model == id;
                        let dl_state = downloading();
                        let is_downloading_this = dl_state
                            .as_ref()
                            .map(|(d, _, _)| d == id)
                            .unwrap_or(false);
                        let any_downloading = dl_state.is_some();
                        let confirming = confirm_dl() == Some(id);
                        let quality = t(&lang, entry.quality_key);
                        let size = size_mb(entry.bytes);
                        rsx! {
                            div { class: "px-4 py-3 space-y-2",
                                div { class: "flex items-center justify-between gap-2",
                                    div { class: "min-w-0",
                                        p { class: "text-sm font-medium text-stone-800", "{id}" }
                                        p { class: "text-xs text-stone-400", "{size} · {quality}" }
                                    }
                                    if is_downloading_this {
                                        {
                                            let (_, done, total) = dl_state.clone().unwrap();
                                            let verifying = total > 0 && done >= total;
                                            let pct = if total > 0 { done * 100 / total } else { 0 };
                                            let label = if verifying {
                                                t(&lang, "whisper-verifying")
                                            } else {
                                                format!("{pct}%")
                                            };
                                            rsx! {
                                                span { class: "text-xs text-ios-orange tabular-nums shrink-0", "{label}" }
                                            }
                                        }
                                    } else if downloaded {
                                        div { class: "flex items-center gap-3 shrink-0",
                                            if is_active {
                                                span { class: "text-xs font-medium text-ios-green",
                                                    {t(&lang, "whisper-active")}
                                                }
                                            } else {
                                                button {
                                                    class: "text-xs text-ios-orange-dark font-medium active:opacity-70",
                                                    onclick: move |_| {
                                                        let _ = db().set_setting(WHISPER_MODEL_KEY, id);
                                                        models_version.set(models_version() + 1);
                                                    },
                                                    {t(&lang, "whisper-set-active")}
                                                }
                                            }
                                            button {
                                                class: "text-xs text-ios-red active:opacity-70",
                                                onclick: move |_| {
                                                    let dir = models::models_dir();
                                                    if let Err(e) = models::delete(&dir, id) {
                                                        dl_error.set(Some(e));
                                                        return;
                                                    }
                                                    if db().get_setting(WHISPER_MODEL_KEY).as_deref() == Some(id) {
                                                        let _ = db().set_setting(WHISPER_MODEL_KEY, "");
                                                    }
                                                    models_version.set(models_version() + 1);
                                                },
                                                {t(&lang, "whisper-delete")}
                                            }
                                        }
                                    } else if !any_downloading && !confirming {
                                        button {
                                            class: "text-xs text-ios-orange-dark font-medium active:opacity-70 shrink-0",
                                            onclick: move |_| confirm_dl.set(Some(id)),
                                            {t(&lang, "whisper-download")}
                                        }
                                    }
                                }
                                if is_downloading_this {
                                    {
                                        let (_, done, total) = dl_state.clone().unwrap();
                                        let pct = if total > 0 { done * 100 / total } else { 0 };
                                        rsx! {
                                            div { class: "h-1.5 bg-stone-100 rounded-full overflow-hidden",
                                                div {
                                                    class: "h-full bg-ios-orange rounded-full",
                                                    style: "width: {pct}%;",
                                                }
                                            }
                                        }
                                    }
                                }
                                if confirming {
                                    {
                                        let confirm_body = t_args(
                                            &lang,
                                            "whisper-confirm-body",
                                            &[("name", id), ("size", &size)],
                                        );
                                        rsx! {
                                            div { class: "rounded-lg bg-stone-50 p-3 space-y-2",
                                                p { class: "text-xs text-stone-600", "{confirm_body}" }
                                                div { class: "flex gap-2",
                                                    button {
                                                        class: crate::ui::kit::CONFIRM_BTN_PRIMARY,
                                                        onclick: move |_| {
                                                            confirm_dl.set(None);
                                                            dl_error.set(None);
                                                            downloading.set(Some((id.to_string(), 0, 0)));
                                                            models::start_background_download(models::models_dir(), id);
                                                        },
                                                        {t(&lang, "whisper-confirm-yes")}
                                                    }
                                                    button {
                                                        class: crate::ui::kit::CONFIRM_BTN_GHOST,
                                                        onclick: move |_| confirm_dl.set(None),
                                                        {t(&lang, "whisper-confirm-cancel")}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "whisper-stored-locally")}
            }
            if cfg!(debug_assertions) {
                WhisperBenchSection {}
            }
        }
    }
}

#[component]
fn WhisperBenchSection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut bench_result: Signal<Option<String>> = use_signal(|| None);
    let mut bench_running = use_signal(|| false);
    let lang = (app.current_lang)();

    let dir = models::models_dir();
    let downloaded = models::list_downloaded(&dir);
    let _ = db;

    rsx! {
        if !downloaded.is_empty() {
            div { class: "space-y-2 pt-2",
                h3 { class: "text-sm font-semibold text-stone-700",
                    {t(&lang, "whisper-bench-title")}
                }
                div { class: "flex flex-wrap gap-2",
                    for id in downloaded {
                        {
                            let lang = lang.clone();
                            rsx! {
                                button {
                                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-200 text-stone-700 active:opacity-70",
                                    disabled: bench_running(),
                                    onclick: move |_| {
                                        let Some(wav) = largest_wav() else {
                                            bench_result.set(Some(t(&lang, "whisper-bench-need-audio")));
                                            return;
                                        };
                                        bench_running.set(true);
                                        bench_result.set(Some(t(&lang, "whisper-bench-running")));
                                        spawn(async move {
                                            let dir = models::models_dir();
                                            let report = match models::model_path(&dir, id) {
                                                Some(model) => {
                                                    crate::services::transcription::whisper::bench(model, wav).await
                                                }
                                                None => Err("Modèle inconnu".to_string()),
                                            };
                                            bench_result.set(Some(match report {
                                                Ok(r) => format!("{id}: {r}"),
                                                Err(e) => format!("{id}: {e}"),
                                            }));
                                            bench_running.set(false);
                                        });
                                    },
                                    "{id}"
                                }
                            }
                        }
                    }
                }
                if let Some(ref r) = bench_result() {
                    p { class: "text-xs text-stone-500 whitespace-pre-wrap break-words", "{r}" }
                }
            }
        }
    }
}

fn largest_wav() -> Option<std::path::PathBuf> {
    let dir = crate::services::audio::output_dir();
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("wav") {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
            best = Some((size, path));
        }
    }
    best.map(|(_, p)| p)
}
