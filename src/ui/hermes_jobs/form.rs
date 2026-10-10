use crate::application::hermes_jobs::{self, frequency_text, Frequency};
use crate::application::i18n::{t, weekday_name};
use crate::infrastructure::hermes::HermesJob;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::{haptic, haptic_prepare};
use crate::ui::chat::hermes_approval::CTA;
use crate::ui::chat::hermes_problem_text;
use crate::ui::icons::{HermesAgentIcon, IconX};
use crate::ui::kit;
use crate::ui::AppState;
use chrono::{Local, Weekday};
use dioxus::prelude::*;
use std::sync::Arc;
use std::time::Duration;

// The sheet slides back down this long before it goes.
const LEAVE: Duration = Duration::from_millis(220);
const DEFAULT_AT: u32 = 7 * 60;
const DEFAULT_HOURS: u32 = 6;
const MAX_HOURS: u32 = 24;
const WEEK: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];
const FIELD: &str = "w-full rounded-[14px] bg-stone-900/5 px-3.5 py-3 text-base text-stone-900 outline-none placeholder-stone-400";
const CHOICE: &str =
    "min-h-[44px] rounded-[14px] text-[15px] transition-colors duration-150";
const ON: &str = "bg-ios-orange text-white font-semibold";
const OFF: &str = "bg-stone-900/5 text-stone-700";

#[derive(Clone, Copy, PartialEq)]
enum Pick {
    Daily,
    Weekdays,
    Weekly,
    Every,
}

// "07:30" from the time field, in minutes after midnight.
fn minutes(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.split_once(':')?;
    Some(h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()?)
}

// Slides the sheet down, then hands over: closed, or the task created.
fn leave(mut leaving: Signal<bool>, then: impl FnOnce() + 'static) {
    if *leaving.peek() {
        return;
    }
    leaving.set(true);
    spawn(async move {
        futures_timer::Delay::new(LEAVE).await;
        then();
    });
}

/// A new task: a name, what Hermes does, and a frequency picked without
/// writing cron. The time field opens the iOS wheel on the iPhone.
#[component]
pub fn NewJobSheet(
    on_close: EventHandler<()>,
    on_created: EventHandler<HermesJob>,
) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut name = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut pick = use_signal(|| Pick::Weekdays);
    let mut at = use_signal(|| DEFAULT_AT);
    let mut day = use_signal(|| Weekday::Mon);
    let mut hours = use_signal(|| DEFAULT_HOURS);
    let mut sending = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let leaving = use_signal(|| false);
    let freq = match pick() {
        Pick::Daily => Frequency::Daily { at: at() },
        Pick::Weekdays => Frequency::Weekdays { at: at() },
        Pick::Weekly => Frequency::Weekly {
            day: day(),
            at: at(),
        },
        Pick::Every => Frequency::EveryHours(hours()),
    };
    let ready =
        !name().trim().is_empty() && !prompt().trim().is_empty() && !sending();
    let create = {
        let (freq, lang) = (freq.clone(), lang.clone());
        move |_| {
            haptic("light");
            sending.set(true);
            error.set(None);
            let (freq, lang) = (freq.clone(), lang.clone());
            spawn(async move {
                let database = db();
                let known = match app.hermes_jobs.peek().as_ref() {
                    Some(Ok(list)) => list.clone(),
                    _ => Vec::new(),
                };
                let made = hermes_jobs::create(
                    &database,
                    name.peek().trim(),
                    prompt.peek().trim(),
                    &freq,
                    &known,
                    &Local,
                )
                .await;
                match made {
                    Ok(job) => {
                        haptic("soft");
                        super::reload(app, &database).await;
                        leave(leaving, move || on_created.call(job));
                    }
                    Err(e) => {
                        sending.set(false);
                        error.set(Some(hermes_problem_text(&lang, &e)));
                    }
                }
            });
        }
    };
    let choice = |p: Pick, key: &'static str| {
        rsx! {
            button {
                class: if pick() == p { "{CHOICE} {ON}" } else { "{CHOICE} {OFF}" },
                onclick: move |_| pick.set(p),
                {t(&lang, key)}
            }
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-stone-900/20 backdrop-fade",
            style: if leaving() { "opacity: 0; transition: opacity 0.22s ease-in;" },
            onclick: move |_| {
                if !sending() {
                    leave(leaving, move || on_close.call(()));
                }
            },
        }
        div {
            class: "glass-panel fixed left-2 right-2 z-50 rounded-[38px] px-5 pt-[18px] pb-5 max-h-[calc(100%-70px)] overflow-y-auto lg:left-[max(0.5rem,calc((100%-30rem)/2))] lg:right-[max(0.5rem,calc((100%-30rem)/2))]",
            style: if leaving() { "bottom: calc(8px + var(--keyboard-inset, 0px)); animation: slideOutDown 0.22s ease-in forwards;" } else { "bottom: calc(8px + var(--keyboard-inset, 0px)); animation: slideInUp 0.26s cubic-bezier(0.3, 1.25, 0.4, 1);" },
            div { class: "flex items-center gap-2.5",
                HermesAgentIcon { size: 28 }
                span { class: "text-[15px] font-semibold text-stone-900", {t(&lang, "hermes-jobs-new")} }
                button {
                    class: "ml-auto glass-disc relative w-9 h-9 -mr-1 flex items-center justify-center rounded-full text-stone-500",
                    "aria-label": t(&lang, "hermes-job-cancel"),
                    onclick: move |_| leave(leaving, move || on_close.call(())),
                    IconX { size: 16 }
                }
            }
            input {
                class: "{FIELD} mt-4",
                placeholder: t(&lang, "hermes-job-name-placeholder"),
                value: "{name}",
                oninput: move |e| name.set(e.value()),
            }
            textarea {
                class: "{FIELD} mt-3 min-h-[96px] resize-none leading-relaxed",
                placeholder: t(&lang, "hermes-job-prompt-placeholder"),
                value: "{prompt}",
                oninput: move |e| prompt.set(e.value()),
            }
            p { class: "{kit::SECTION_LABEL} mt-4 mb-2", {t(&lang, "hermes-job-frequency")} }
            div { class: "grid grid-cols-2 gap-2",
                {choice(Pick::Daily, "hermes-job-freq-daily")}
                {choice(Pick::Weekdays, "hermes-job-freq-weekdays")}
                {choice(Pick::Weekly, "hermes-job-freq-weekly")}
                {choice(Pick::Every, "hermes-job-freq-every")}
            }
            if pick() == Pick::Weekly {
                div { class: "mt-3 flex gap-1.5",
                    for w in WEEK {
                        button {
                            key: "{w}",
                            class: if day() == w { "flex-1 h-11 rounded-full text-[15px] {ON}" } else { "flex-1 h-11 rounded-full text-[15px] {OFF}" },
                            "aria-label": weekday_name(&lang, w),
                            onclick: move |_| day.set(w),
                            {weekday_name(&lang, w).chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}
                        }
                    }
                }
            }
            if pick() == Pick::Every {
                div { class: "mt-3 flex items-center justify-between rounded-[18px] bg-stone-900/5 px-4 min-h-[52px]",
                    span { class: "text-[15px] text-stone-800", {t(&lang, "hermes-job-every-label")} }
                    span { class: "flex items-center gap-2",
                        button {
                            class: "w-11 h-11 rounded-full bg-warm-white text-xl text-stone-700",
                            onclick: move |_| hours.set(hours().saturating_sub(1).max(1)),
                            "−"
                        }
                        span { class: "w-14 text-center text-[17px] font-semibold tabular-nums", "{hours} h" }
                        button {
                            class: "w-11 h-11 rounded-full bg-warm-white text-xl text-stone-700",
                            onclick: move |_| hours.set((hours() + 1).min(MAX_HOURS)),
                            "+"
                        }
                    }
                }
            } else {
                label { class: "mt-3 flex items-center justify-between rounded-[18px] bg-stone-900/5 px-4 min-h-[52px]",
                    span { class: "text-[15px] text-stone-800", {t(&lang, "hermes-job-at")} }
                    input {
                        r#type: "time",
                        class: "h-9 px-3 rounded-lg bg-stone-900/[0.06] text-[17px] text-stone-900 tabular-nums outline-none",
                        value: format!("{:02}:{:02}", at() / 60, at() % 60),
                        oninput: move |e| {
                            if let Some(m) = minutes(&e.value()) {
                                at.set(m);
                            }
                        },
                    }
                }
            }
            p { class: "mt-3 text-sm text-stone-500 text-center", {frequency_text(&lang, &freq)} }
            if let Some(err) = error() {
                p { class: "mt-2 text-[13px] leading-snug text-ios-red-dark text-center", "{err}" }
            }
            button {
                class: "{CTA} mt-4 bg-ios-orange text-white",
                disabled: !ready,
                onpointerdown: move |_| haptic_prepare("light"),
                onclick: create,
                {t(&lang, if sending() { "hermes-job-creating" } else { "hermes-job-create" })}
            }
        }
    }
}
