mod account;
mod backup;
mod connections;
mod general;
mod intelligence;
mod privacy;
mod shortcuts;
mod spaces;
mod storage;
mod tab_pins;
mod transcription;

use crate::application::backup as backup_service;
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{
    IconArrowsClockwise, IconCaretRight, IconFloppyDisk, IconGlobeSimple,
    IconHardDrives, IconHeadCircuit, IconKeyboard, IconMicrophone,
    IconPlugsConnected, IconShieldCheck, IconUserCircle, IconUsersThree,
};
use crate::ui::kit::SECTION_LABEL;
use crate::ui::{AppState, SettingsSection, View};
use account::AccountSettings;
use backup::BackupSettings;
use connections::ConnectionsSettings;
use dioxus::prelude::*;
use general::GeneralSettings;
use intelligence::IntelligenceSettings;
use privacy::PrivacySettings;
use shortcuts::ShortcutsSettings;
use spaces::SpacesSettings;
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

    let mut premium = use_signal(|| None::<bool>);
    use_effect(move || {
        spawn(async move {
            let database = db();
            if let Some(client) =
                crate::infrastructure::backend::BackendClient::from_db(
                    &database,
                )
            {
                if let Ok(acc) = client.account(&database).await {
                    premium.set(Some(acc.premium));
                }
            }
        });
    });

    rsx! {
        div { class: "space-y-5 pb-20",
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

            SettingsGroup {
                button {
                    class: ROW,
                    onclick: move |_| {
                        app.previous_view.set(Some(View::Settings));
                        app.view.set(View::SettingsSection(SettingsSection::Account));
                    },
                    div { class: ICON_TILE, IconUserCircle { size: 17 } }
                    span { class: "flex-1 text-sm font-medium text-stone-800",
                        {t(&lang, "settings-section-account")}
                    }
                    if let Some(is_premium) = premium() {
                        span {
                            class: if is_premium { PREMIUM_PILL } else { FREE_PILL },
                            {t(&lang, if is_premium { "account-premium" } else { "account-free" })}
                        }
                    }
                    span { class: "shrink-0 text-stone-400", IconCaretRight { size: 14 } }
                }
                IconRow { label: t(&lang, "settings-section-connections"), section: SettingsSection::Connections,
                    div { class: ICON_TILE, IconPlugsConnected { size: 17 } }
                }
                IconRow { label: t(&lang, "settings-section-spaces"), section: SettingsSection::Spaces,
                    div { class: ICON_TILE, IconUsersThree { size: 17 } }
                }
            }

            SettingsGroup { label: Some(t(&lang, "settings-group-intelligence")),
                IconRow { label: t(&lang, "settings-section-intelligence"), section: SettingsSection::Intelligence,
                    div { class: ICON_TILE, IconHeadCircuit { size: 17 } }
                }
                IconRow { label: t(&lang, "settings-section-transcription"), section: SettingsSection::Transcription,
                    div { class: ICON_TILE, IconMicrophone { size: 17 } }
                }
            }

            SettingsGroup { label: Some(t(&lang, "settings-group-data")),
                button {
                    class: ROW,
                    onclick: move |_| {
                        app.previous_view.set(Some(View::Settings));
                        app.view.set(View::SyncPairing);
                    },
                    div { class: ICON_TILE, IconArrowsClockwise { size: 17 } }
                    span { class: "flex-1 text-sm font-medium text-stone-800",
                        {t(&lang, "settings-sync-title")}
                    }
                    span { class: "shrink-0 text-stone-400", IconCaretRight { size: 14 } }
                }
                IconRow { label: t(&lang, "settings-storage-title"), section: SettingsSection::Storage,
                    div { class: ICON_TILE, IconHardDrives { size: 17 } }
                }
                IconRow { label: t(&lang, "settings-backup-title"), section: SettingsSection::Backup,
                    div { class: ICON_TILE, IconFloppyDisk { size: 17 } }
                }
                IconRow { label: t(&lang, "settings-section-privacy"), section: SettingsSection::Privacy,
                    div { class: ICON_TILE, IconShieldCheck { size: 17 } }
                }
            }

            SettingsGroup { label: Some(t(&lang, "settings-group-general")),
                IconRow { label: t(&lang, "settings-section-general"), section: SettingsSection::General,
                    div { class: ICON_TILE, IconGlobeSimple { size: 17 } }
                }
                if cfg!(target_os = "macos") {
                    IconRow { label: t(&lang, "settings-section-shortcuts"), section: SettingsSection::Shortcuts,
                        div { class: ICON_TILE, IconKeyboard { size: 17 } }
                    }
                }
            }
        }
    }
}

const ICON_TILE: &str =
    "w-8 h-8 rounded-[10px] bg-ios-orange-50 text-ios-orange flex items-center justify-center shrink-0";
const ROW: &str =
    "pressable w-full min-h-[44px] flex items-center gap-3 px-4 py-3 text-left hover:bg-stone-50 active:bg-stone-100 transition-colors duration-150";
const PREMIUM_PILL: &str =
    "text-[11px] font-semibold text-ios-orange-dark bg-ios-orange-50 border border-ios-orange-100 px-2 py-0.5 rounded-full mr-2";
const FREE_PILL: &str =
    "text-[11px] font-medium text-stone-500 bg-stone-100 px-2 py-0.5 rounded-full mr-2";

#[component]
fn SettingsGroup(
    #[props(default)] label: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            if let Some(label) = label {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), "{label}" }
            }
            div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                {children}
            }
        }
    }
}

// One navigable settings row: the icon tile (passed as children), the label, then a chevron.
#[component]
fn IconRow(
    label: String,
    section: SettingsSection,
    children: Element,
) -> Element {
    let mut app: AppState = use_context();
    rsx! {
        button {
            class: ROW,
            onclick: move |_| {
                app.previous_view.set(Some(View::Settings));
                app.view.set(View::SettingsSection(section));
            },
            {children}
            span { class: "flex-1 text-sm font-medium text-stone-800", "{label}" }
            span { class: "shrink-0 text-stone-400", IconCaretRight { size: 14 } }
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
        SettingsSection::Account => rsx! {
            AccountSettings {}
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
        SettingsSection::Connections => rsx! {
            ConnectionsSettings {}
        },
        SettingsSection::Privacy => rsx! {
            PrivacySettings {}
        },
        SettingsSection::Shortcuts => rsx! {
            ShortcutsSettings {}
        },
        SettingsSection::Spaces => rsx! {
            SpacesSettings {}
        },
    }
}
