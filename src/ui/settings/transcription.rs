use crate::application::i18n::{t, t_args};
use crate::application::transcribe_audio::save_soniox_key;
use crate::application::transcription_dictionary;
use crate::domain::{Dictionary, DictionaryEntry};
use crate::infrastructure::persistence::settings_repo::{
    STT_PROVIDER_KEY, WHISPER_MODEL_KEY,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::transcription::models;
use crate::infrastructure::transcription::SttProvider;
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
            DictionarySection {}

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

            // Which engine actually runs on the next note, spelled out. The tab
            // alone was not enough: on cloud, a local model name stayed on screen
            // and there was no way to tell which one would transcribe.
            ActiveEngineBanner { provider }

            if provider() == SttProvider::Soniox {
                SonioxKeyForm {}
            } else {
                WhisperModelsSection {}
            }
        }
    }
}

#[component]
fn DictionarySection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut entries =
        use_signal(|| transcription_dictionary::load(&db()).entries().to_vec());
    let mut heard = use_signal(String::new);
    let mut correct = use_signal(String::new);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-3",
            h2 { class: "text-lg font-semibold text-stone-900",
                {t(&lang, "dictionary-title")}
            }
            p { class: "text-xs text-stone-500", {t(&lang, "dictionary-hint")} }

            div { class: "flex gap-2",
                input {
                    class: crate::ui::kit::INPUT,
                    placeholder: t(&lang, "dictionary-heard-placeholder"),
                    value: "{heard}",
                    oninput: move |evt| heard.set(evt.value()),
                }
                input {
                    class: crate::ui::kit::INPUT,
                    placeholder: t(&lang, "dictionary-correct-placeholder"),
                    value: "{correct}",
                    oninput: move |evt| correct.set(evt.value()),
                }
            }
            button {
                class: crate::ui::kit::BTN_PRIMARY,
                disabled: correct().trim().is_empty(),
                onclick: move |_| {
                    let target = correct().trim().to_string();
                    if target.is_empty() {
                        return;
                    }
                    let source = heard().trim().to_string();
                    let source = if source.is_empty() { target.clone() } else { source };
                    let mut next = entries();
                    next.push(DictionaryEntry::new(source, target));
                    let dict = Dictionary::from_entries(next);
                    let _ = transcription_dictionary::save(&db(), &dict);
                    entries.set(dict.entries().to_vec());
                    heard.set(String::new());
                    correct.set(String::new());
                },
                {t(&lang, "dictionary-add")}
            }

            if entries().is_empty() {
                p { class: "text-xs text-stone-400 text-center py-2",
                    {t(&lang, "dictionary-empty")}
                }
            } else {
                div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                    for (idx, entry) in entries().into_iter().enumerate() {
                        div { class: "px-4 py-3 flex items-center justify-between gap-2",
                            div { class: "min-w-0",
                                p { class: "text-sm font-medium text-stone-800 truncate",
                                    "{entry.correct}"
                                }
                                if entry.heard != entry.correct {
                                    p { class: "text-xs text-stone-400 truncate", "{entry.heard}" }
                                }
                            }
                            button {
                                class: "text-xs text-ios-red active:opacity-70 shrink-0",
                                onclick: move |_| {
                                    let mut next = entries();
                                    if idx >= next.len() {
                                        return;
                                    }
                                    next.remove(idx);
                                    let dict = Dictionary::from_entries(next);
                                    let _ = transcription_dictionary::save(&db(), &dict);
                                    entries.set(dict.entries().to_vec());
                                },
                                {t(&lang, "dictionary-delete")}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// States the engine that will transcribe the next note, and refuses to claim one
/// is ready when it is not: no key on cloud, or no model downloaded on local.
#[component]
fn ActiveEngineBanner(provider: Signal<SttProvider>) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let (name, blocker) = match provider() {
        SttProvider::Soniox => {
            let has_key = db()
                .get_setting("soniox_api_key")
                .is_some_and(|k| !k.trim().is_empty());
            (
                t(&lang, "settings-stt-cloud"),
                (!has_key).then(|| t(&lang, "stt-error-soniox-key")),
            )
        }
        SttProvider::WhisperLocal => {
            let dir = models::models_dir();
            let active =
                db().get_setting(WHISPER_MODEL_KEY).unwrap_or_default();
            let ready =
                !active.is_empty() && models::is_downloaded(&dir, &active);
            let label = if ready {
                format!("{} · {active}", t(&lang, "settings-stt-local"))
            } else {
                t(&lang, "settings-stt-local")
            };
            (label, (!ready).then(|| t(&lang, "stt-error-no-model")))
        }
    };

    rsx! {
        div {
            class: if blocker.is_some() {
                "rounded-xl border border-ios-red/40 bg-ios-red/5 px-4 py-3"
            } else {
                "rounded-xl border border-stone-200 bg-warm-white px-4 py-3"
            },
            p { class: "text-xs text-stone-500", {t(&lang, "settings-stt-active-label")} }
            p { class: "text-sm font-medium text-stone-800 mt-0.5", "{name}" }
            if let Some(ref reason) = blocker {
                p { class: "text-xs text-ios-red mt-1", "{reason}" }
            }
        }
    }
}

/// What the last save attempt produced. A key is never "saved" on its own: it is
/// either confirmed by Soniox or rejected with the reason.
#[derive(Clone, PartialEq)]
enum KeyState {
    Idle,
    Checking,
    Valid,
    Invalid(String),
}

#[component]
fn SonioxKeyForm() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut soniox_key =
        use_signal(|| db().get_setting("soniox_api_key").unwrap_or_default());
    let mut state = use_signal(|| KeyState::Idle);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-4",
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-soniox-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: t(&lang, "settings-soniox-placeholder"),
                    value: "{soniox_key}",
                    oninput: move |evt| {
                        soniox_key.set(evt.value());
                        state.set(KeyState::Idle);
                    },
                }
            }

            button {
                class: crate::ui::kit::BTN_PRIMARY,
                disabled: state() == KeyState::Checking || soniox_key().trim().is_empty(),
                onclick: move |_| {
                    let key = soniox_key().trim().to_string();
                    state.set(KeyState::Checking);
                    spawn(async move {
                        match save_soniox_key(&db(), &key).await {
                            Ok(()) => state.set(KeyState::Valid),
                            Err(e) => state.set(KeyState::Invalid(e)),
                        }
                    });
                },
                match state() {
                    KeyState::Checking => t(&lang, "settings-key-checking"),
                    _ => t(&lang, "settings-save"),
                }
            }

            match state() {
                KeyState::Valid => rsx! {
                    p { class: "text-xs text-ios-green", {t(&lang, "settings-key-valid")} }
                },
                KeyState::Invalid(ref reason) => rsx! {
                    div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                        p { class: "text-xs text-ios-red font-medium",
                            {t(&lang, "settings-key-invalid")}
                        }
                        p { class: "text-xs text-stone-500 mt-1 break-words", "{reason}" }
                    }
                },
                _ => rsx! {},
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
        }
    }
}
