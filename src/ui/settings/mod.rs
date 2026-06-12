mod backup;
mod general;
mod intelligence;
mod privacy;
mod shortcuts;
mod storage;
mod transcription;

use crate::db::Database;
use crate::services::backup as backup_service;
use crate::services::i18n::t;
use crate::ui::{AppState, SettingsSection, View};
use backup::BackupSettings;
use dioxus::prelude::*;
use general::GeneralSettings;
use intelligence::IntelligenceSettings;
use privacy::PrivacySettings;
use shortcuts::ShortcutsSettings;
use std::sync::Arc;
use storage::StorageSettings;
use transcription::TranscriptionSettings;

#[component]
pub fn SettingsView() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let restore_error = use_signal(backup_service::take_restore_error);
    let restored_pending =
        db().get_setting("sync_restored_pending").as_deref() == Some("true");
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-4 pb-20",
            if let Some(ref err) = restore_error() {
                div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                    p { class: "text-xs font-medium text-ios-red mb-1",
                        {t(&lang, "settings-restore-error-title")}
                    }
                    p { class: "text-xs text-stone-600 break-words", "{err}" }
                }
            }
            if restored_pending {
                div { class: "rounded-xl border border-ios-orange/40 bg-ios-orange/5 p-3",
                    p { class: "text-xs text-stone-700 mb-2",
                        {t(&lang, "settings-repair-invite")}
                    }
                    button {
                        class: crate::ui::kit::BTN_PRIMARY,
                        onclick: move |_| {
                            app.previous_view.set(Some(View::Settings));
                            app.view.set(View::SyncPairing);
                        },
                        {t(&lang, "settings-repair-button")}
                    }
                }
            }
            div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                SectionRow {
                    label: t(&lang, "settings-section-general"),
                    section: SettingsSection::General,
                }
                SectionRow {
                    label: t(&lang, "settings-section-intelligence"),
                    section: SettingsSection::Intelligence,
                }
                SectionRow {
                    label: t(&lang, "settings-section-transcription"),
                    section: SettingsSection::Transcription,
                }
                button {
                    class: "w-full min-h-[44px] flex items-center justify-between px-4 py-3 text-left",
                    onclick: move |_| {
                        app.previous_view.set(Some(View::Settings));
                        app.view.set(View::SyncPairing);
                    },
                    span { class: "text-sm font-medium text-stone-800",
                        {t(&lang, "settings-sync-title")}
                    }
                    span { class: "inline-block w-1.5 h-1.5 border-r-2 border-t-2 border-stone-400 rotate-45" }
                }
                SectionRow {
                    label: t(&lang, "settings-storage-title"),
                    section: SettingsSection::Storage,
                }
                SectionRow {
                    label: t(&lang, "settings-backup-title"),
                    section: SettingsSection::Backup,
                }
                SectionRow {
                    label: t(&lang, "settings-section-privacy"),
                    section: SettingsSection::Privacy,
                }
                if cfg!(target_os = "macos") {
                    SectionRow {
                        label: t(&lang, "settings-section-shortcuts"),
                        section: SettingsSection::Shortcuts,
                    }
                }
            }
        }
    }
}

#[component]
fn SectionRow(label: String, section: SettingsSection) -> Element {
    let mut app: AppState = use_context();
    rsx! {
        button {
            class: "w-full min-h-[44px] flex items-center justify-between px-4 py-3 text-left",
            onclick: move |_| {
                app.previous_view.set(Some(View::Settings));
                app.view.set(View::SettingsSection(section));
            },
            span { class: "text-sm font-medium text-stone-800", "{label}" }
            span { class: "inline-block w-1.5 h-1.5 border-r-2 border-t-2 border-stone-400 rotate-45" }
        }
    }
}

#[component]
pub fn SettingsSectionView() -> Element {
    let app: AppState = use_context();
    let View::SettingsSection(section) = (app.view)() else {
        return rsx! {};
    };
    match section {
        SettingsSection::General => rsx! {
            GeneralSettings {}
        },
        SettingsSection::Intelligence => rsx! {
            IntelligenceSettings {}
        },
        SettingsSection::Transcription => rsx! {
            TranscriptionSettings {}
        },
        SettingsSection::Storage => rsx! {
            StorageSettings {}
        },
        SettingsSection::Backup => rsx! {
            BackupSettings {}
        },
        SettingsSection::Privacy => rsx! {
            PrivacySettings {}
        },
        SettingsSection::Shortcuts => rsx! {
            ShortcutsSettings {}
        },
    }
}
