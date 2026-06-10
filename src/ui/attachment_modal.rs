use crate::ui::icons::IconX;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn AttachmentModal() -> Element {
    let mut app: AppState = use_context();
    let modal = (app.attachment_modal)();

    let Some(attachment) = modal else {
        return rsx! {};
    };

    let date = attachment.imported_at[..10].to_string();

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/40",
            onclick: move |_| app.attachment_modal.set(None),
        }
        div {
            class: "fixed inset-x-0 bottom-0 z-50 bg-warm-white rounded-t-2xl shadow-lg flex flex-col",
            style: "max-height: 85vh; animation: slideInUp 0.2s ease-out;",
            div { class: "flex items-center justify-between px-4 py-3 border-b border-stone-100",
                div { class: "flex-1 min-w-0",
                    p { class: "text-sm font-semibold text-stone-900 truncate", "{attachment.filename}" }
                    p { class: "text-xs text-stone-400", "{date}" }
                }
                button {
                    class: "w-11 h-11 flex items-center justify-center rounded-full active:bg-stone-100 text-stone-500 -mr-2",
                    onclick: move |_| app.attachment_modal.set(None),
                    IconX { size: 20 }
                }
            }
            div { class: "flex-1 overflow-y-auto px-4 safe-py-3",
                pre { class: "text-sm text-stone-800 whitespace-pre-wrap font-sans",
                    "{attachment.content_text}"
                }
            }
        }
    }
}
