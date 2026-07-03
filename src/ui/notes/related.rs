use crate::application::i18n::t;
use crate::application::related::{related_notes, RelatedNote};
use crate::ui::icons::IconNote;
use crate::ui::notes::dates::format_absolute_short;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

/// "Notes liées" at the bottom of NoteDetail: the 3 semantically closest notes,
/// resolved fully locally (stored chunk vector -> LanceDB). Hidden when the note
/// is not embedded yet or nothing is close enough - the section never shows noise.
#[component]
pub fn RelatedSection(local_note_id: Signal<String>) -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    let related = use_resource(move || {
        let id = local_note_id();
        async move {
            if id.is_empty() {
                return Vec::new();
            }
            related_notes(&id).await.unwrap_or_default()
        }
    });

    let items: Vec<RelatedNote> = related().unwrap_or_default();
    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "border-t border-stone-100 mt-4 pt-3",
            p { class: "text-xs font-medium text-stone-400 uppercase tracking-wide px-1 mb-2",
                {t(&lang, "note-related-title")}
            }
            for item in items {
                {
                    let target = item.note_id.clone();
                    let date = format_absolute_short(&item.created_at, &lang);
                    rsx! {
                        button {
                            key: "{item.note_id}",
                            class: "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-xl text-left hover:bg-stone-100 active:bg-stone-100 transition-colors duration-150",
                            onclick: move |_| {
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
                                div { class: "truncate text-sm font-medium text-stone-800", "{item.label}" }
                                if !date.is_empty() {
                                    div { class: "truncate text-xs text-stone-400", "{date}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
