use crate::application::hermes_message::Attachment;
use crate::ui::sf_icon::SfIcon;
use dioxus::prelude::*;

const TAG: &str = "inline-flex items-center gap-1.5 pl-2.5 pr-2 py-1 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium";
// A skill called on purpose stands out: lit orange, as the suggested pill
// it came from turns when tapped.
const LIT: &str = "inline-flex items-center gap-1.5 pl-2.5 pr-2 py-1 rounded-full bg-ios-orange text-white shadow-card text-xs font-medium";

/// What goes with the next question; the cross takes one back.
#[component]
pub fn AttachmentChips(list: Signal<Vec<Attachment>>) -> Element {
    let mut list = list;
    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5 px-2 pb-2",
            for (i, a) in list().iter().enumerate() {
                {
                    let skill = matches!(a, Attachment::Skill { .. });
                    rsx! {
                        span {
                            key: "{i}{a.label()}",
                            class: if skill { LIT } else { TAG },
                            style: "animation: fadeInUp 0.15s ease-out;",
                            if skill {
                                SfIcon { name: "sparkles", size: 12 }
                            }
                            span { class: "max-w-[200px] truncate", "{a.label()}" }
                            button {
                                class: if skill { "flex text-white/80" } else { "flex text-ios-orange-dark" },
                                onclick: move |_| {
                                    list.write().remove(i);
                                },
                                SfIcon { name: "xmark", size: 10 }
                            }
                        }
                    }
                }
            }
        }
    }
}
