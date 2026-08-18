// The card a scheduled reminder leaves in the chat. A sentence saying "reminder created"
// leaves nothing to open, correct or undo, and nothing survives the scroll - which is the
// standing complaint against assistants that schedule things. This card is the proof.
//
// The due label is read from the persisted row at render time rather than carried in the
// message, so the card and the note's own reminder line can never drift apart.

use crate::application::i18n::{reminder_due_label, t, t_args};
use crate::application::tools::ReminderCard as CardData;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconArrowUpRight, IconBell};
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn ReminderCard(
    card: CardData,
    cancelled: bool,
    on_cancel: EventHandler<()>,
) -> Element {
    let app = use_context::<AppState>();
    let lang = (app.current_lang)();

    let row = Database::open().ok().and_then(|db| {
        db.reminders_for_note(&card.note_id)
            .into_iter()
            .find(|r| r.id == card.row_id)
    });
    let due = row
        .as_ref()
        .map(|r| reminder_due_label(&lang, r))
        .unwrap_or_default();
    // macOS has no event sheet to present, so it opens Calendar on the day instead - same
    // fallback the note's own reminder line uses.
    let due_parts = row.as_ref().map(|r| {
        (r.due_year, r.due_month, r.due_day, r.due_hour, r.due_minute)
    });
    let event_id = row.map(|r| r.reminder_id).unwrap_or_default();

    let on_note = t_args(
        &lang,
        "reminder-card-on-note",
        &[("note", card.note_title.as_str())],
    );

    if cancelled {
        return rsx! {
            div { class: "mt-2 p-3 bg-warm-white border border-stone-200 rounded-xl opacity-70",
                div { class: "flex items-start gap-2.5",
                    span { class: "shrink-0 mt-0.5 text-stone-400", IconBell { size: 18 } }
                    div {
                        p { class: "text-sm font-semibold text-stone-500 line-through", "{card.title}" }
                        p { class: "mt-0.5 text-xs text-stone-400", "{due}" }
                    }
                }
                span { class: "inline-flex items-center mt-2.5 px-2.5 py-1 rounded-full text-xs font-medium bg-stone-100 text-stone-500",
                    {t(&lang, "reminder-card-cancelled")}
                }
            }
        };
    }

    rsx! {
        div { class: "mt-2 p-3 bg-warm-white border border-ios-orange/30 rounded-xl",
            div { class: "flex items-start gap-2.5",
                span { class: "shrink-0 mt-0.5 text-ios-orange-dark", IconBell { size: 18 } }
                div { class: "min-w-0",
                    p { class: "text-sm font-semibold text-stone-800 break-words", "{card.title}" }
                    p { class: "mt-0.5 text-xs text-stone-500 break-words", "{on_note}" }
                }
            }
            div { class: "mt-2.5 flex gap-2",
                button {
                    class: "flex-1 min-h-[44px] flex items-center justify-center gap-1.5 rounded-lg bg-ios-orange/10 text-ios-orange-dark text-xs font-medium active:bg-ios-orange/25",
                    onclick: move |_| {
                        #[cfg(target_os = "ios")]
                        {
                            let event_id = event_id.clone();
                            spawn(async move {
                                crate::infrastructure::platform::ios::reminders::present_event(event_id).await;
                            });
                        }
                        #[cfg(target_os = "macos")]
                        if let Some((y, m, d, h, min)) = due_parts {
                            crate::infrastructure::platform::macos::open_calendar_at(y, m, d, h, min);
                        }
                        #[cfg(not(target_os = "ios"))]
                        let _ = &event_id;
                        #[cfg(not(target_os = "macos"))]
                        let _ = due_parts;
                    },
                    span { "{due}" }
                    IconArrowUpRight { size: 14 }
                }
                button {
                    class: "shrink-0 min-h-[44px] px-4 rounded-lg bg-ios-red/10 text-ios-red text-xs font-medium active:opacity-70",
                    onclick: move |_| on_cancel.call(()),
                    {t(&lang, "reminder-card-undo")}
                }
            }
        }
    }
}
