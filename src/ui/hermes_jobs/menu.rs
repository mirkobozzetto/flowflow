use crate::application::hermes_jobs::JobAction;
use crate::application::i18n::t;
use crate::ui::icons::{IconPause, IconPlay, IconTrash};
use crate::ui::kit;
use dioxus::prelude::*;

/// A task's actions. The same buttons feed the native iOS menu of a task's
/// screen (`data-native-*`) and draw the web menu everywhere else.
#[component]
pub fn JobMenuItems(
    paused: bool,
    lang: String,
    on_pick: EventHandler<JobAction>,
) -> Element {
    let (toggle, key, symbol) = if paused {
        (JobAction::Resume, "hermes-job-resume", "play")
    } else {
        (JobAction::Pause, "hermes-job-pause", "pause")
    };
    rsx! {
        button {
            class: kit::MENU_ITEM,
            "data-native-action": "run",
            "data-native-symbol": "play",
            onclick: move |_| on_pick.call(JobAction::RunNow),
            IconPlay { size: 16 }
            {t(&lang, "hermes-job-run")}
        }
        button {
            class: kit::MENU_ITEM,
            "data-native-action": "toggle",
            "data-native-symbol": symbol,
            onclick: move |_| on_pick.call(toggle),
            if paused {
                IconPlay { size: 16 }
            } else {
                IconPause { size: 16 }
            }
            {t(&lang, key)}
        }
        div { class: kit::MENU_SEP }
        button {
            class: kit::MENU_ITEM_DANGER,
            "data-native-action": "delete",
            "data-native-symbol": "trash",
            "data-native-destructive": "",
            onclick: move |_| on_pick.call(JobAction::Delete),
            IconTrash { size: 16 }
            {t(&lang, "hermes-job-delete")}
        }
    }
}
