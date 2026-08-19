use crate::application::i18n::t;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn SearchFilters() -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let f = (app.note_filters)();

    rsx! {
        div { class: "absolute right-1.5 top-1/2 -translate-y-1/2 flex items-center gap-0.5",
            FilterToggle {
                on: f.voice,
                label: t(&lang, "note-filter-voice"),
                on_click: move |_| {
                    let mut n = (app.note_filters)();
                    n.voice = !n.voice;
                    app.note_filters.set(n);
                },
                IconMicrophone { size: 14 }
            }
            FilterToggle {
                on: f.reminder,
                label: t(&lang, "note-filter-reminder"),
                on_click: move |_| {
                    let mut n = (app.note_filters)();
                    n.reminder = !n.reminder;
                    app.note_filters.set(n);
                },
                IconBell { size: 14 }
            }
            FilterToggle {
                on: f.document,
                label: t(&lang, "note-filter-document"),
                on_click: move |_| {
                    let mut n = (app.note_filters)();
                    n.document = !n.document;
                    app.note_filters.set(n);
                },
                IconFileArrowUp { size: 14 }
            }
            FilterToggle {
                on: f.thread,
                label: t(&lang, "note-filter-thread"),
                on_click: move |_| {
                    let mut n = (app.note_filters)();
                    n.thread = !n.thread;
                    app.note_filters.set(n);
                },
                IconCardsThree { size: 14 }
            }
        }
    }
}

#[component]
fn FilterToggle(
    on: bool,
    label: String,
    on_click: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "w-7 h-7 flex items-center justify-center rounded-lg transition-colors duration-150",
            // Lighter than the magnifying glass (stone-400), so a toggle at rest
            // never competes with the field; lit, it takes the exact orange the
            // note card already uses for the same marker.
            class: if on {
                "text-ios-orange-dark"
            } else {
                "text-stone-300 hover:text-stone-500"
            },
            title: "{label}",
            onclick: move |e| on_click.call(e),
            {children}
        }
    }
}
