use crate::application::hermes_delivery::DeliveryError;
use crate::application::i18n::t;
use crate::ui::chat::hermes_problem_text;
use crate::ui::icons::IconBell;
use crate::ui::settings::push_problem_text;
use dioxus::prelude::*;

/// What went wrong turning FlowFlow on, in one sentence.
pub fn delivery_problem_text(lang: &str, e: &DeliveryError) -> String {
    match e {
        DeliveryError::Push(e) => push_problem_text(lang, e),
        DeliveryError::Hermes(e) => hermes_problem_text(lang, e),
    }
}

/// "Receive in FlowFlow" and its switch, the size of iOS' own.
#[component]
pub fn FlowFlowSwitch(
    on: bool,
    busy: bool,
    lang: String,
    class: String,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "{class} w-full flex items-center gap-3 text-left disabled:opacity-60",
            role: "switch",
            "aria-checked": on,
            disabled: busy,
            onclick: move |_| onclick.call(()),
            span { class: "text-ios-orange-dark", IconBell { size: 20 } }
            span { class: "flex-1 text-[15px] text-stone-900", {t(&lang, "hermes-job-flowflow")} }
            span { class: "relative w-[51px] h-[31px] shrink-0 rounded-full bg-stone-900/15", "aria-hidden": "true",
                span { class: "tools-switch-on absolute inset-0 rounded-full bg-ios-orange", "data-on": on }
                span { class: "tools-switch-knob absolute top-[2px] left-[2px] w-[27px] h-[27px] rounded-full bg-white shadow-card", "data-on": on }
            }
        }
    }
}
