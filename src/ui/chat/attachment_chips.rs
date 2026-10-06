use crate::application::hermes_message::Attachment;
use crate::ui::icons::IconX;
use dioxus::prelude::*;

/// What goes with the next question, in the note tag style; the cross
/// takes one back.
#[component]
pub fn AttachmentChips(list: Signal<Vec<Attachment>>) -> Element {
    let mut list = list;
    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5 px-2 pb-2",
            for (i, a) in list().iter().enumerate() {
                span {
                    key: "{i}{a.label()}",
                    class: "inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                    span { class: "max-w-[200px] truncate", "{a.label()}" }
                    button {
                        class: "ml-0.5 text-ios-orange-dark",
                        onclick: move |_| {
                            list.write().remove(i);
                        },
                        IconX { size: 12 }
                    }
                }
            }
        }
    }
}
