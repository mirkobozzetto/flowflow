use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::action_card::ActionResultCard;
use crate::ui::chat::actions::{find_saved_note, md_to_html, save_as_note};
use crate::ui::chat::models::ChatSource;
use crate::ui::chat::sources_accordion::SourcesAccordion;
use crate::ui::chat::trace_accordion::TraceAccordion;
use crate::ui::clipboard::copy_text;
use crate::ui::icons::{IconCheck, IconCopy, IconDotsThree, IconFloppyDisk};
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn BotBubble(
    text: String,
    sources: Vec<ChatSource>,
    conversation_id: Option<String>,
    #[props(default)] trace: Option<crate::application::rag::RagTrace>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut copied = use_signal(|| false);
    let mut actions_open = use_signal(|| false);
    let text_for_lookup = text.clone();
    let saved_id = use_memo(move || {
        let _ = (app.notes_version)();
        let _ = (app.sync_data_version)();
        let folder = crate::ui::chat::actions::scope_save_folder(
            &db(),
            &(app.chat_scope)(),
        );
        find_saved_note(&db(), folder.as_deref(), &text_for_lookup)
    });
    let copy_label = t(&lang, "common-copy");
    let copied_label = t(&lang, "common-copied");
    let save_label = t(&lang, "chat-save-note");
    let saved_label = t(&lang, "chat-saved");
    let text_for_copy = text.clone();
    let text_for_save = text.clone();
    let lang_for_save = lang.clone();
    let web_sources_for_save: Vec<ChatSource> = sources
        .iter()
        .filter(|s| s.url.is_some())
        .cloned()
        .collect();
    let conv_for_open = conversation_id.clone();

    // An executed-action reply (one line + link) renders as the same clean card as in a note,
    // without the save/copy/sources chrome. Heuristic, so it also catches the agent autonomously
    // running a connector mid-chat. Stable per message, so the early return is hook-safe.
    if crate::application::intent::looks_like_action(&text) {
        return rsx! {
            div {
                class: "flex justify-start items-start gap-2",
                style: "animation: fadeInUp 0.15s ease-out;",
                img {
                    src: asset!("/assets/flowflow-icon-300.png"),
                    class: "w-5 h-5 mt-1 object-contain rounded-full shrink-0",
                    alt: "FlowFlow",
                }
                div { class: "max-w-[85%]",
                    ActionResultCard { text: text.clone() }
                }
            }
        };
    }

    rsx! {
        div {
            class: "group flex justify-start items-start gap-2",
            style: "animation: fadeInUp 0.15s ease-out;",
            img {
                src: asset!("/assets/flowflow-icon-300.png"),
                class: "w-5 h-5 mt-1 object-contain rounded-full shrink-0",
                alt: "FlowFlow",
            }
            div { class: "min-w-0 max-w-[85%] py-1",

                div {
                    class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                    dangerous_inner_html: md_to_html(&text),
                }
                div { class: "relative flex justify-end mt-1.5",
                    button {
                        class: "pressable w-8 h-8 flex items-center justify-center rounded-full text-stone-400 hover:text-stone-600 hover:bg-stone-100",
                        aria_label: "Actions",
                        aria_expanded: "{actions_open()}",
                        onclick: move |_| actions_open.set(!actions_open()),
                        IconDotsThree { size: 18 }
                    }
                    if actions_open() {
                        div { class: "absolute right-0 top-9 z-50 min-w-[180px] bg-warm-white border border-stone-200 rounded-xl shadow-menu p-1 menu-anchor",
                    button {
                        class: if saved_id().is_some() {
                            "w-full flex items-center gap-2.5 px-3 py-2.5 min-h-[40px] rounded-lg text-sm text-ios-green text-left"
                        } else {
                            "w-full flex items-center gap-2.5 px-3 py-2.5 min-h-[40px] rounded-lg text-sm text-stone-800 text-left hover:bg-stone-100 active:bg-stone-100"
                        },
                        onclick: move |_| {
                            actions_open.set(false);
                            if let Some(nid) = saved_id() {
                                let folder = db()
                                    .folders_for_note(&nid)
                                    .ok()
                                    .and_then(|f| f.first().map(|f| f.id.clone()));
                                app.detail_folder_id.set(folder);
                                app.previous_view.set(Some(View::Chat {
                                    conversation_id: conv_for_open.clone(),
                                }));
                                app.view.set(View::NoteDetail { note_id: nid });
                                return;
                            }
                            let database = db();
                            let folder = crate::ui::chat::actions::scope_save_folder(
                                &database,
                                &(app.chat_scope)(),
                            );
                            if save_as_note(
                                &database,
                                folder.as_deref(),
                                text_for_save.clone(),
                                vec!["chat".to_string()],
                                &web_sources_for_save,
                                &lang_for_save,
                            )
                            .is_ok()
                            {
                                app.notes_version
                                    .set((app.notes_version)() + 1);
                            }
                        },
                        if saved_id().is_some() {
                            IconCheck { size: 12 }
                            "{saved_label}"
                        } else {
                            IconFloppyDisk { size: 12 }
                            "{save_label}"
                        }
                    }
                    button {
                        class: if copied() {
                            "w-full flex items-center gap-2.5 px-3 py-2.5 min-h-[40px] rounded-lg text-sm text-ios-green text-left"
                        } else {
                            "w-full flex items-center gap-2.5 px-3 py-2.5 min-h-[40px] rounded-lg text-sm text-stone-800 text-left hover:bg-stone-100 active:bg-stone-100"
                        },
                        onclick: move |_| {
                            actions_open.set(false);
                            if copied() {
                                return;
                            }
                            copy_text(&text_for_copy);
                            copied.set(true);
                            spawn(async move {
                                futures_timer::Delay::new(
                                    std::time::Duration::from_millis(1500),
                                )
                                .await;
                                copied.set(false);
                            });
                        },
                        if copied() {
                            IconCheck { size: 12 }
                            "{copied_label}"
                        } else {
                            IconCopy { size: 12 }
                            "{copy_label}"
                        }
                    }
                        }
                    }
                }
                if !sources.is_empty() {
                    SourcesAccordion {
                        sources: sources.clone(),
                        conversation_id: conversation_id,
                    }
                }
                if let Some(tr) = trace.filter(|_| {
                    db().get_setting("debug_trace").as_deref() == Some("1")
                }) {
                    TraceAccordion { trace: tr }
                }
            }
        }
    }
}
