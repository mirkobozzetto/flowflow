use crate::application::i18n::t;
use crate::application::related::{linked_notes, set_link_state, LinkedNote};
use crate::ui::icons::{IconNote, IconPushPin, IconX};
use crate::ui::notes::dates::format_absolute_short;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

/// "Notes liées" at the bottom of NoteDetail: ONE merged list (similarity is
/// symmetric, both stored directions fuse). Stored rows are labeled and
/// pin/dismiss-able via long-press; a note without computed links falls back
/// to the live v2 search. Hidden entirely when there is nothing to show.
#[component]
pub fn RelatedSection(local_note_id: Signal<String>) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let refresh = use_signal(|| 0u32);
    let menu_for = use_signal(|| None::<String>);
    let pressed = use_signal(|| None::<(String, u64)>);
    let press_seq = use_signal(|| 0u64);
    let suppress_click = use_signal(|| false);

    let related = use_resource(move || {
        let id = local_note_id();
        let _tick = refresh();
        async move {
            if id.is_empty() {
                return Vec::new();
            }
            linked_notes(&id).await.unwrap_or_default()
        }
    });

    let links = related().unwrap_or_default();
    if links.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "border-t border-stone-200/70 mt-4 pt-3",
            p { class: "text-xs font-medium text-stone-400 uppercase tracking-[0.08em] px-1 mb-2",
                {t(&lang, "note-related-title")}
            }
            for item in links {
                LinkRow {
                    key: "{item.note_id}",
                    item,
                    local_note_id,
                    menu_for,
                    pressed,
                    press_seq,
                    suppress_click,
                    refresh,
                }
            }
        }
    }
}

#[component]
fn LinkRow(
    item: LinkedNote,
    local_note_id: Signal<String>,
    menu_for: Signal<Option<String>>,
    pressed: Signal<Option<(String, u64)>>,
    press_seq: Signal<u64>,
    suppress_click: Signal<bool>,
    refresh: Signal<u32>,
) -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let row_key = item.note_id.clone();
    let menu_open = menu_for() == Some(row_key.clone());
    let date = format_absolute_short(&item.created_at, &lang);
    let subtitle = item.why.clone().unwrap_or(date);
    // Pair state: set_link_state touches both stored directions.
    let (src, dst) = (local_note_id(), item.note_id.clone());

    let down_key = row_key.clone();
    let actionable = item.actionable;
    let target = item.note_id.clone();

    rsx! {
        button {
            class: "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-xl text-left hover:bg-stone-100 active:bg-stone-100 transition-colors duration-150",
            onpointerdown: move |_| {
                if !actionable {
                    return;
                }
                let seq = press_seq() + 1;
                press_seq.set(seq);
                let mut pressed = pressed;
                let mut menu_for = menu_for;
                let mut suppress_click = suppress_click;
                pressed.set(Some((down_key.clone(), seq)));
                let key = down_key.clone();
                spawn(async move {
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(450),
                    )
                    .await;
                    if pressed() == Some((key.clone(), seq)) {
                        menu_for.set(Some(key.clone()));
                        suppress_click.set(true);
                        pressed.set(None);
                    }
                });
            },
            onpointerup: move |_| {
                let mut pressed = pressed;
                pressed.set(None);
            },
            onpointercancel: move |_| {
                let mut pressed = pressed;
                pressed.set(None);
            },
            onpointerleave: move |_| {
                let mut pressed = pressed;
                pressed.set(None);
            },
            onclick: move |_| {
                let mut suppress_click = suppress_click;
                let mut menu_for = menu_for;
                if suppress_click() {
                    suppress_click.set(false);
                    return;
                }
                if menu_for().is_some() {
                    menu_for.set(None);
                    return;
                }
                // Back from the opened note returns HERE, not to the list.
                let source = local_note_id();
                app.previous_view.set(Some(View::NoteDetail { note_id: source }));
                // Note -> note needs a bounce through NotesList: Dioxus keeps
                // the mounted NoteDetail otherwise and its state never resets.
                // spawn_forever, NOT spawn: this component unmounts on the
                // first set() and a scope-bound task would die with it,
                // stranding the user on the list.
                app.view.set(View::NotesList);
                let target = target.clone();
                dioxus::core::spawn_forever(async move {
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(30),
                    )
                    .await;
                    app.view.set(View::NoteDetail { note_id: target });
                });
            },
            span { class: "w-[22px] h-[22px] shrink-0 flex items-center justify-center text-stone-500",
                IconNote { size: 20 }
            }
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-1.5 min-w-0",
                    if item.pinned {
                        span { class: "shrink-0 text-stone-500",
                            IconPushPin { size: 13 }
                        }
                    }
                    div { class: "truncate text-sm font-medium text-stone-800", "{item.display}" }
                }
                if !subtitle.is_empty() {
                    div { class: "truncate text-xs text-stone-400", "{subtitle}" }
                }
            }
        }
        if menu_open {
            div { class: "flex items-center gap-2 px-3 pb-2",
                button {
                    class: "flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-stone-100 text-xs font-medium text-stone-700 active:bg-stone-200",
                    onclick: {
                        let src = src.clone();
                        let dst = dst.clone();
                        let pinned = item.pinned;
                        move |_| {
                            let mut menu_for = menu_for;
                            let mut refresh = refresh;
                            let next = if pinned { "active" } else { "pinned" };
                            let _ = set_link_state(&src, &dst, next);
                            menu_for.set(None);
                            refresh.set(refresh() + 1);
                        }
                    },
                    IconPushPin { size: 14 }
                    if item.pinned {
                        {t(&lang, "note-related-unpin")}
                    } else {
                        {t(&lang, "note-related-pin")}
                    }
                }
                button {
                    class: "flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-stone-100 text-xs font-medium text-ios-red active:bg-stone-200",
                    onclick: {
                        let src = src.clone();
                        let dst = dst.clone();
                        move |_| {
                            let mut menu_for = menu_for;
                            let mut refresh = refresh;
                            let _ = set_link_state(&src, &dst, "dismissed");
                            menu_for.set(None);
                            refresh.set(refresh() + 1);
                        }
                    },
                    IconX { size: 14 }
                    {t(&lang, "note-related-dismiss")}
                }
            }
        }
    }
}
