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
            class: "fixed inset-x-0 bottom-0 z-50 bg-white rounded-t-2xl shadow-lg flex flex-col",
            style: "max-height: 85vh; animation: slideInUp 0.2s ease-out;",
            div { class: "flex items-center justify-between px-4 py-3 border-b border-gray-100",
                div { class: "flex-1 min-w-0",
                    p { class: "text-sm font-semibold text-gray-900 truncate", "{attachment.filename}" }
                    p { class: "text-xs text-gray-400", "{date}" }
                }
                button {
                    class: "w-11 h-11 flex items-center justify-center rounded-full active:bg-gray-100 text-gray-500 -mr-2",
                    onclick: move |_| app.attachment_modal.set(None),
                    IconX { size: 20 }
                }
            }
            div { class: "flex-1 overflow-y-auto px-4 py-3",
                pre { class: "text-sm text-gray-800 whitespace-pre-wrap font-sans",
                    "{attachment.content_text}"
                }
            }
        }
    }
}
