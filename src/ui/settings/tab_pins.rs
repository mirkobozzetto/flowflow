// Per-tab pin editor for one armed spreadsheet: checkboxes over its tabs, default all.
// Checking a subset writes a binding v2 `sheet` pin; all (or none pinned) = whole workbook.

use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use std::sync::Arc;

/// The pin editor's state: (spreadsheet_id, every tab, checked set).
pub type TabPicker = (String, Vec<String>, BTreeSet<String>);

/// The tabs currently pinned for one armed spreadsheet (empty = whole workbook).
pub fn pinned_tabs(db: &Database, spreadsheet_id: &str) -> Vec<String> {
    crate::application::connector_module::binding_entries(db)
        .into_iter()
        .filter(|e| e.spreadsheet_id == spreadsheet_id)
        .filter_map(|e| e.sheet)
        .collect()
}

/// Fetch the picker state for one spreadsheet: every tab, with the pinned set checked
/// (or everything, when the whole workbook is armed).
pub async fn load_tab_picker(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<TabPicker, String> {
    let tabs = crate::application::connector_module::list_spreadsheet_tabs(
        db,
        spreadsheet_id,
    )
    .await?;
    let pins: BTreeSet<String> =
        pinned_tabs(db, spreadsheet_id).into_iter().collect();
    let checked = if pins.is_empty() {
        tabs.iter().cloned().collect()
    } else {
        pins
    };
    Ok((spreadsheet_id.to_string(), tabs, checked))
}

#[component]
pub fn TabPinEditor(
    picker: Signal<Option<TabPicker>>,
    bindings: Signal<Vec<(String, String)>>,
    status: Signal<Option<String>>,
) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let Some((_, tabs, checked)) = picker() else {
        return rsx! {};
    };

    rsx! {
        div { class: "rounded-xl bg-warm-white border border-stone-200 p-3 space-y-2",
            style: "animation: fadeInUp 0.18s ease-out;",
            p { class: "text-[10px] text-stone-400 leading-relaxed",
                {t(&lang, "connections-tabs-hint")}
            }
            div { class: "divide-y divide-stone-100",
                for tab in tabs.clone() {
                    button {
                        key: "{tab}",
                        class: "w-full min-h-[40px] px-1 flex items-center gap-2 text-left active:bg-stone-50 transition-colors",
                        onclick: {
                            let tab = tab.clone();
                            move |_| {
                                let Some((id, tabs, mut checked)) = picker() else { return; };
                                if !checked.remove(&tab) {
                                    checked.insert(tab.clone());
                                }
                                picker.set(Some((id, tabs, checked)));
                            }
                        },
                        span {
                            class: if checked.contains(&tab) {
                                "w-4 h-4 shrink-0 rounded border border-ios-green bg-ios-green text-white text-[10px] flex items-center justify-center"
                            } else {
                                "w-4 h-4 shrink-0 rounded border border-stone-300"
                            },
                            if checked.contains(&tab) { "✓" }
                        }
                        p { class: "text-xs text-stone-700 truncate", "{tab}" }
                    }
                }
            }
            button {
                class: crate::ui::kit::PILL_PRIMARY,
                disabled: checked.is_empty(),
                onclick: move |_| {
                    let Some((id, tabs, checked)) = picker() else { return; };
                    // Every tab checked = whole workbook: the pin clears and the
                    // binding stays v1 (downgrade-safe).
                    let pins: Vec<String> = if checked.len() == tabs.len() {
                        Vec::new()
                    } else {
                        checked.iter().cloned().collect()
                    };
                    match crate::application::connector_module::set_sheet_pins(&db(), &id, &pins) {
                        Ok(()) => {
                            bindings.set(crate::application::connector_module::current_bindings(&db()));
                            picker.set(None);
                            status.set(None);
                        }
                        Err(e) => status.set(Some(e)),
                    }
                },
                {t(&lang, "connections-tabs-apply")}
            }
        }
    }
}
