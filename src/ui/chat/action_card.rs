use crate::ui::chat::md_to_html;
use crate::ui::icons::IconCheck;
use dioxus::prelude::*;

// Clean result card for an executed note-action: a green check plus the agent's one-line
// confirmation, with any returned link rendered as a prominent pill. Shared by the note detail
// and the chat bubble so an action looks identical wherever it runs.
// ponytail: the link is styled inline via [&_a] utilities; upgrade path = pull the link onto its
// own full-width CTA row if a bigger tap target is wanted.
#[component]
pub fn ActionResultCard(text: String) -> Element {
    rsx! {
        div {
            class: "mt-2 p-3 bg-warm-white border border-ios-orange/30 rounded-xl flex items-start gap-2.5",
            span { class: "shrink-0 mt-0.5 text-ios-orange-dark",
                IconCheck { size: 18 }
            }
            div {
                class: "text-sm text-stone-700 leading-relaxed break-words [&_a]:inline-flex [&_a]:items-center [&_a]:mt-1 [&_a]:px-3 [&_a]:py-1.5 [&_a]:bg-ios-blue/10 [&_a]:text-ios-blue [&_a]:font-medium [&_a]:rounded-lg [&_a]:no-underline [&_a]:break-all active:[&_a]:bg-ios-blue/20",
                dangerous_inner_html: md_to_html(&text),
            }
        }
    }
}
