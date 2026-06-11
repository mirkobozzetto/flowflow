use crate::db::Database;
use crate::models::{NewTextNote, NoteReminder, ReminderIntent};
use crate::services::i18n::{reminder_due_label, t, t_args};
use crate::ui::icons::{IconArrowUpRight, IconBell, IconTrash, IconX};
use crate::ui::AppState;
use chrono::{Local, NaiveTime, Timelike};
use dioxus::prelude::*;
use std::sync::Arc;
use std::time::Duration;

#[component]
pub fn ActiveReminders(local_note_id: Signal<String>) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut confirm_delete_reminder: Signal<Option<String>> =
        use_signal(|| None);

    let detail_reminders: Vec<NoteReminder> = {
        let id = local_note_id();
        let _ = (app.notes_version)();
        if id.is_empty() {
            Vec::new()
        } else {
            db().reminders_for_note(&id)
                .into_iter()
                .filter(|r| r.state == "active")
                .collect()
        }
    };

    rsx! {
        if !detail_reminders.is_empty() {
            div { class: "mt-2 px-1 flex flex-col gap-1.5",
                for r in detail_reminders.iter() {
                    {
                        let due = reminder_due_label(&lang, r);
                        let del_label = t(&lang, "reminder-delete");
                        let cancel_label = t(&lang, "reminder-cancel");
                        let row_id = r.id.clone();
                        let event_id_open = r.reminder_id.clone();
                        let event_id_del = r.reminder_id.clone();
                        let confirming = confirm_delete_reminder() == Some(row_id.clone());
                        rsx! {
                            div { class: "flex items-center gap-1.5",
                                button {
                                    class: "flex-1 flex items-center gap-2 min-h-[44px] px-3 py-2 rounded-xl bg-ios-orange/10 text-ios-orange-dark text-xs font-medium active:bg-ios-orange/25",
                                    onclick: move |_| {
                                        #[cfg(target_os = "ios")]
                                        {
                                            let event_id_open = event_id_open.clone();
                                            spawn(async move {
                                                crate::platform::ios::reminders::present_event(event_id_open).await;
                                            });
                                        }
                                        #[cfg(not(target_os = "ios"))]
                                        let _ = &event_id_open;
                                    },
                                    IconBell { size: 15 }
                                    span { class: "flex-1 text-left", "{due}" }
                                    IconArrowUpRight { size: 14 }
                                }
                                if confirming {
                                    button {
                                        class: "shrink-0 min-h-[44px] px-3 rounded-xl text-xs font-medium text-white bg-ios-red active:opacity-70",
                                        onclick: {
                                            let row_id = row_id.clone();
                                            let event_id_del = event_id_del.clone();
                                            move |_| {
                                                confirm_delete_reminder.set(None);
                                                let _ = db().delete_note_reminder(&row_id);
                                                app.notes_version.set((app.notes_version)() + 1);
                                                #[cfg(target_os = "ios")]
                                                {
                                                    let event_id_del = event_id_del.clone();
                                                    spawn(async move {
                                                        if let Err(e) = crate::platform::ios::reminders::remove_event(event_id_del).await {
                                                            eprintln!("[reminder] remove_event failed: {e}");
                                                        }
                                                    });
                                                }
                                                #[cfg(not(target_os = "ios"))]
                                                let _ = &event_id_del;
                                            }
                                        },
                                        "{del_label}"
                                    }
                                    button {
                                        class: "shrink-0 min-h-[44px] px-2 text-xs font-medium text-stone-500 active:opacity-70",
                                        onclick: move |_| confirm_delete_reminder.set(None),
                                        "{cancel_label}"
                                    }
                                } else {
                                    button {
                                        class: "shrink-0 min-h-[44px] p-2 text-stone-400 active:text-ios-red",
                                        onclick: {
                                            let row_id = row_id.clone();
                                            move |_| confirm_delete_reminder.set(Some(row_id.clone()))
                                        },
                                        IconTrash { size: 16 }
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

#[component]
pub fn ReminderSuggestions(
    mut local_note_id: Signal<String>,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
    initial_content: String,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut detected_reminders: Signal<Vec<ReminderIntent>> =
        use_signal(Vec::new);
    let mut reminders_checked = use_signal(String::new);
    let mut detecting_reminders = use_signal(|| false);
    let mut reminder_feedback: Signal<Option<String>> = use_signal(|| None);

    let reminder_initial_content = initial_content.clone();
    use_effect(move || {
        let c = content();
        if c.trim().chars().count() < 8 {
            if !detected_reminders.peek().is_empty() {
                detected_reminders.set(Vec::new());
            }
            return;
        }
        if *reminders_checked.peek() == c {
            return;
        }
        let nid = local_note_id();
        if c == reminder_initial_content
            && !nid.is_empty()
            && db().note_has_active_reminder(&nid)
        {
            reminders_checked.set(c);
            return;
        }
        spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(1200)).await;
            if *content.peek() != c {
                return;
            }
            if *reminders_checked.peek() == c || *detecting_reminders.peek() {
                return;
            }
            detecting_reminders.set(true);
            match crate::services::llm::LlmClient::from_db(&db()) {
                Ok(ai) => {
                    if let Ok(intents) =
                        ai.extract_reminders(&c, Local::now()).await
                    {
                        let normalized: Vec<ReminderIntent> = intents
                            .into_iter()
                            .map(|mut it| {
                                if let Some(d) = it.resolved_date() {
                                    it.date =
                                        Some(d.format("%Y-%m-%d").to_string());
                                }
                                let tt = it.resolved_time();
                                it.time = Some(format!(
                                    "{:02}:{:02}",
                                    tt.hour(),
                                    tt.minute()
                                ));
                                it.time_end =
                                    it.resolved_time_end().map(|end| {
                                        format!(
                                            "{:02}:{:02}",
                                            end.hour(),
                                            end.minute()
                                        )
                                    });
                                it
                            })
                            .collect();
                        reminders_checked.set(c.clone());
                        detected_reminders.set(normalized);
                    }
                }
                Err(_) => {
                    reminders_checked.set(c.clone());
                }
            }
            detecting_reminders.set(false);
        });
    });

    let detected = detected_reminders();
    let pending_idx: Vec<usize> = detected
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            let nid = local_note_id();
            nid.is_empty()
                || !db().reminder_exists_by_intent_hash(&nid, &i.intent_hash())
        })
        .map(|(idx, _)| idx)
        .collect();
    let title_label = t(&lang, "reminder-detected-title");
    let confirm_label = t(&lang, "reminder-confirm");
    let range_add_label = t(&lang, "reminder-range-add");
    let range_to_label = t(&lang, "reminder-range-to");
    let lang_badge = lang.clone();

    rsx! {
        if !pending_idx.is_empty() {
                div { class: "mt-3 p-3 bg-ios-orange/10 border border-ios-orange/30 rounded-xl",
                div { class: "flex items-center justify-between mb-2",
                    span { class: "text-xs font-semibold text-ios-orange-dark", "{title_label}" }
                    button {
                        class: "text-stone-400 active:opacity-70",
                        onclick: move |_| {
                            reminders_checked.set(content.peek().clone());
                            detected_reminders.set(Vec::new());
                        },
                        IconX { size: 14 }
                    }
                }
                div { class: "space-y-2",
                    for idx in pending_idx {
                        {
                            let intent = detected[idx].clone();
                            let action = intent.action.clone();
                            let date_val = intent.date.clone().unwrap_or_default();
                            let time_val = intent.time.clone().unwrap_or_default();
                            let end_val = intent.time_end.clone().unwrap_or_default();
                            let has_end = intent.time_end.is_some();
                            let default_end = {
                                use chrono::Duration as ChronoDuration;
                                NaiveTime::parse_from_str(&time_val, "%H:%M")
                                    .ok()
                                    .map(|tm| tm + ChronoDuration::hours(1))
                                    .map(|tm| format!("{:02}:{:02}", tm.hour(), tm.minute()))
                                    .unwrap_or_else(|| "10:00".to_string())
                            };
                            let is_rec = intent.recurrence.is_some();
                            let lang_cb = lang_badge.clone();
                            let confirm = confirm_label.clone();
                            let range_add = range_add_label.clone();
                            let range_to = range_to_label.clone();
                            rsx! {
                                div { class: "bg-white/40 rounded-lg p-2.5",
                                    p { class: "text-sm text-stone-800 mb-1.5", "{action}" }
                                    div { class: "flex items-center gap-2 flex-wrap",
                                        input {
                                            r#type: "date",
                                            class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                            value: "{date_val}",
                                            oninput: move |e| {
                                                detected_reminders.write()[idx].date = Some(e.value());
                                            },
                                        }
                                        input {
                                            r#type: "time",
                                            class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                            value: "{time_val}",
                                            oninput: move |e| {
                                                detected_reminders.write()[idx].time = Some(e.value());
                                            },
                                        }
                                        if has_end {
                                            span { class: "text-xs text-stone-500", "{range_to}" }
                                            input {
                                                r#type: "time",
                                                class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                                value: "{end_val}",
                                                oninput: move |e| {
                                                    detected_reminders.write()[idx].time_end = Some(e.value());
                                                },
                                            }
                                            button {
                                                class: "text-stone-400 active:opacity-70",
                                                onclick: move |_| {
                                                    detected_reminders.write()[idx].time_end = None;
                                                },
                                                IconX { size: 12 }
                                            }
                                        } else {
                                            button {
                                                class: "text-xs font-medium text-ios-orange-dark active:opacity-70",
                                                onclick: move |_| {
                                                    detected_reminders.write()[idx].time_end = Some(default_end.clone());
                                                },
                                                "{range_add}"
                                            }
                                        }
                                        if is_rec {
                                            span { class: "text-xs text-ios-orange-dark", "↻" }
                                        }
                                        button {
                                        class: "shrink-0 self-center text-xs font-medium text-white bg-ios-orange-dark px-3 py-1.5 rounded-full active:opacity-70",
                                        onclick: move |_| {
                                            let intent = detected_reminders.peek()[idx].clone();
                                            let lang2 = lang_cb.clone();
                                            let database = db();
                                            spawn(async move {
                                                reminder_feedback.set(Some(t(&lang2, "reminder-creating")));
                                                let nid = {
                                                    let cur = local_note_id();
                                                    if cur.is_empty() {
                                                        let tt = title();
                                                        let new = NewTextNote {
                                                            title: if tt.is_empty() { None } else { Some(tt) },
                                                            content: content(),
                                                            tags: tags(),
                                                        };
                                                        match database.create_text_note(&new) {
                                                            Ok(created) => {
                                                                if let Some(ref fid) = (app.detail_folder_id)() {
                                                                    let _ = database.add_note_to_folder(&created.id, fid);
                                                                }
                                                                local_note_id.set(created.id.clone());
                                                                app.current_note_id.set(Some(created.id.clone()));
                                                                app.notes_version.set((app.notes_version)() + 1);
                                                                created.id
                                                            }
                                                            Err(e) => {
                                                                reminder_feedback.set(Some(t_args(&lang2, "reminder-failed", &[("error", &e)])));
                                                                return;
                                                            }
                                                        }
                                                    } else {
                                                        cur
                                                    }
                                                };
                                                let res = crate::services::reminders::schedule(database, nid, intent.clone()).await;
                                                use crate::services::reminders::ScheduleResult;
                                                match res {
                                                    ScheduleResult::Created => {
                                                        app.notes_version.set((app.notes_version)() + 1);
                                                        reminder_feedback.set(Some(t(&lang2, "reminder-created")));
                                                    }
                                                    ScheduleResult::Duplicate => reminder_feedback.set(Some(t(&lang2, "reminder-duplicate"))),
                                                    ScheduleResult::AccessDenied => reminder_feedback.set(Some(t(&lang2, "reminder-denied"))),
                                                    ScheduleResult::Unsupported => reminder_feedback.set(Some("iOS only".to_string())),
                                                    ScheduleResult::Failed(e) => reminder_feedback.set(Some(t_args(&lang2, "reminder-failed", &[("error", &e)]))),
                                                }
                                            });
                                        },
                                        "{confirm}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }
        }
        if let Some(msg) = reminder_feedback() {
            div {
                class: "mt-2 px-3 py-2 bg-ios-orange/10 rounded-lg text-xs text-stone-600",
                "{msg}"
            }
        }
    }
}
