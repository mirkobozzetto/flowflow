use crate::db::Database;
use crate::models::Attachment;
use crate::services::embed::delete_attachment_embeddings;
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn AttachmentSection(
    attachments: Vec<Attachment>,
    confirm_delete_att: Signal<Option<String>>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut confirm_delete_att = confirm_delete_att;
    let lang = (app.current_lang)();
    let label_confirm = t(&lang, "attachment-confirm-delete");
    let label_cancel = t(&lang, "attachment-cancel");
    let label_delete = t(&lang, "attachment-delete");

    rsx! {
        for att in attachments.iter() {
            {
                let att_card = att.clone();
                let att_name = att.filename.clone();
                let att_name_confirm = att.filename.clone();
                let att_id_del = att.id.clone();
                let att_id_confirm = att.id.clone();
                let date = att.imported_at[..10].to_string();
                let is_confirming = confirm_delete_att() == Some(att.id.clone());
                rsx! {
                    div {
                        key: "{att.id}",
                        class: "relative overflow-hidden bg-warm-white border border-stone-200 rounded-xl h-[60px] flex items-center px-3 mt-2 active:bg-stone-50",
                        onclick: move |_| {
                            if !is_confirming {
                                app.attachment_modal.set(Some(att_card.clone()));
                            }
                        },
                        div { class: "w-9 h-9 rounded-lg bg-warm-white border border-ios-orange/20 flex items-center justify-center flex-shrink-0",
                            IconFileArrowUp { size: 18 }
                        }
                        div { class: "flex-1 min-w-0 ml-3",
                            p { class: "text-sm font-medium text-stone-900 truncate leading-tight", "{att_name}" }
                            p { class: "text-xs text-stone-400 leading-tight", "{date}" }
                        }
                        button {
                            class: "w-11 h-11 flex items-center justify-center rounded-full text-stone-400 -mr-1 active:bg-stone-100",
                            onclick: {
                                move |evt: Event<MouseData>| {
                                    evt.stop_propagation();
                                    confirm_delete_att.set(Some(att_id_confirm.clone()));
                                }
                            },
                            IconTrash { size: 18 }
                        }
                        if is_confirming {
                            div {
                                class: "absolute inset-0 z-10 flex items-center px-3 bg-warm-white/95 backdrop-blur-sm",
                                onclick: move |evt| evt.stop_propagation(),
                                div { class: "flex-1 min-w-0 mr-3",
                                    p { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide leading-tight", "{label_confirm}" }
                                    p { class: "text-sm font-medium text-stone-600 truncate leading-tight", "{att_name_confirm}" }
                                }
                                div { class: "flex items-center gap-2",
                                    button {
                                        class: "h-9 px-4 flex items-center justify-center text-sm font-medium text-stone-900 bg-stone-100 rounded-full active:bg-stone-200",
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            confirm_delete_att.set(None);
                                        },
                                        "{label_cancel}"
                                    }
                                    button {
                                        class: "h-9 px-4 flex items-center justify-center text-sm font-medium text-white bg-ios-red rounded-full active:opacity-80",
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            let db = db();
                                            let id = att_id_del.clone();
                                            let _ = db.delete_attachment(&id);
                                            delete_attachment_embeddings(id);
                                            confirm_delete_att.set(None);
                                            app.attachments_version.set((app.attachments_version)() + 1);
                                        },
                                        "{label_delete}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
