use crate::application::i18n::t;
use crate::domain::Note;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconArrowUpRight, IconBell};
use crate::ui::{AppState, RowMenu, View};
use dioxus::prelude::*;
use std::sync::Arc;

/// Finger travel, in CSS pixels, past which a press is a scroll, not a hold.
const PRESS_SLOP: f64 = 10.0;
/// How long a finger must stay down before the note action sheet opens.
const LONG_PRESS_MS: u64 = 450;

#[component]
pub fn NoteCard(note: Note) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let note_id = note.id.clone();
    let lang = (app.current_lang)();
    // Same press bookkeeping as the related-notes rows (related.rs): a counter
    // invalidates the pending timer when the finger lifts or the list scrolls.
    let mut press_seq = use_signal(|| 0u32);
    let mut pressed = use_signal(|| None::<u32>);
    let mut press_origin = use_signal(|| (0.0f64, 0.0f64));
    let mut suppress_click = use_signal(|| false);
    let press_id = note.id.clone();
    let menu_id = note.id.clone();

    let title = note
        .title
        .clone()
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));

    let cleaned_preview = crate::application::ai::plain_preview(&note.content);
    let preview = if cleaned_preview.chars().count() > 120 {
        format!(
            "{}...",
            crate::application::ai::char_prefix(&cleaned_preview, 120)
        )
    } else {
        cleaned_preview
    };

    let date = &note.created_at[..10];
    let _ = (app.notes_version)();
    let has_audio = db().has_any_audio(&note.id);
    let active_reminders: Vec<_> = db()
        .reminders_for_note(&note.id)
        .into_iter()
        .filter(|r| r.state == "active")
        .collect();
    let has_reminder = !active_reminders.is_empty();
    let reminder_due = active_reminders
        .first()
        .map(|r| crate::application::i18n::reminder_due_label(&lang, r));

    let folder_name = db()
        .folders_for_note(&note.id)
        .ok()
        .and_then(|f| f.first().map(|f| f.name.clone()));

    let author =
        crate::application::authorship::author_label(&db.peek(), &note);

    // The pressed card stays lit above the dimming veil the list draws.
    let picked = (app.row_menu)() == Some(RowMenu::Note(note.id.clone()));

    rsx! {
        div {
            class: "bg-warm-white p-4 border border-stone-200 rounded-xl mb-2.5 lg:mb-0 lg:h-full cursor-pointer break-inside-avoid shadow-card hover:border-stone-300 hover:shadow-lift hover:-translate-y-px transition-[box-shadow,transform,border-color] duration-180",
            // Sits between the veil and the menu: lit enough to read as selected,
            // dimmer than the panel that owns the focus.
            class: if picked { "relative z-20 shadow-lift border-stone-300 brightness-90" } else { "" },
            onpointerdown: move |evt| {
                let p = evt.client_coordinates();
                press_origin.set((p.x, p.y));
                let seq = press_seq() + 1;
                press_seq.set(seq);
                pressed.set(Some(seq));
                let id = press_id.clone();
                spawn(async move {
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(LONG_PRESS_MS),
                    )
                    .await;
                    if pressed() == Some(seq) {
                        pressed.set(None);
                        suppress_click.set(true);
                        app.row_menu_at.set(press_origin());
                        app.row_menu.set(Some(RowMenu::Note(id)));
                    }
                });
            },
            // Desktop equivalent of the hold, per the platform convention.
            oncontextmenu: move |evt| {
                evt.prevent_default();
                pressed.set(None);
                let p = evt.client_coordinates();
                app.row_menu_at.set((p.x, p.y));
                app.row_menu.set(Some(RowMenu::Note(menu_id.clone())));
            },
            onpointerup: move |_| pressed.set(None),
            onpointercancel: move |_| pressed.set(None),
            onpointerleave: move |_| pressed.set(None),
            // A finger always drifts a little; only real travel (a scroll) cancels.
            onpointermove: move |evt| {
                if pressed().is_none() {
                    return;
                }
                let p = evt.client_coordinates();
                let (ox, oy) = press_origin();
                if (p.x - ox).abs() > PRESS_SLOP || (p.y - oy).abs() > PRESS_SLOP {
                    pressed.set(None);
                }
            },
            onclick: move |_| {
                if suppress_click() {
                    suppress_click.set(false);
                    return;
                }
                if (app.row_menu)().is_some() {
                    app.row_menu.set(None);
                    return;
                }
                app.show_folder_picker.set(false);
                app.view.set(View::NoteDetail { note_id: note_id.clone() });
            },
            div { class: "flex justify-between items-center mb-2",
                h3 { class: "font-semibold text-base text-stone-900", "{title}" }
                div { class: "flex items-center gap-1.5 shrink-0",
                    if note.sources_json.is_some() {
                        span { class: "text-ios-orange-dark", IconArrowUpRight { size: 14 } }
                    }
                    if has_reminder {
                        span { class: "text-ios-orange-dark", IconBell { size: 14 } }
                    }
                    if has_audio {
                        div { class: "w-2 h-2 rounded-full bg-ios-orange/50" }
                    }
                }
            }
            if !preview.is_empty() {
                p { class: "text-stone-600 text-sm mb-2 line-clamp-2", "{preview}" }
            }
            if !note.tags.is_empty() {
                div { class: "flex flex-wrap gap-1 mb-1.5",
                    for tag in note.tags.iter() {
                        span { class: "px-2 py-0.5 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                            "{tag}"
                        }
                    }
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "text-stone-500 text-xs", "{date}" }
                if let Some(ref fname) = folder_name {
                    span { class: "text-xs text-stone-400", "·" }
                    span { class: "text-xs text-ios-orange-dark", "{fname}" }
                }
                if let Some(ref author) = author {
                    span { class: "px-1.5 py-0.5 rounded-full bg-warm-white border border-stone-200 text-stone-500 text-xs font-medium truncate max-w-[10rem]",
                        "{author}"
                    }
                }
            }
            if let Some(ref due) = reminder_due {
                div { class: "flex items-center gap-1 mt-1 text-ios-orange-dark text-xs font-medium",
                    IconBell { size: 12 }
                    span { "{due}" }
                }
            }
        }
    }
}
