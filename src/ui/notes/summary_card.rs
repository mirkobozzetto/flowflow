use crate::application::i18n::t;
use crate::application::summary::summarize_note;
use crate::domain::summary::{source_hash, Moment, Summary, SummaryItem};
use crate::infrastructure::audio;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::haptic;
use crate::ui::icons::*;
use crate::ui::kit;
use crate::ui::notes::playback::{NotePlayer, Playback};
use crate::ui::notes::reading::clock;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const COPIED_MS: u64 = 1500;
const PART_LABEL: &str =
    "text-[11px] font-semibold uppercase tracking-wide text-stone-400 mb-1.5";

/// The summary's four parts with their headings, in reading order.
fn parts<'a>(
    summary: &'a Summary,
    lang: &str,
) -> [(String, &'a [SummaryItem]); 4] {
    [
        (t(lang, "summary-key-points"), &summary.key_points),
        (t(lang, "summary-decisions"), &summary.decisions),
        (t(lang, "summary-actions"), &summary.actions),
        (t(lang, "summary-questions"), &summary.questions),
    ]
}

fn as_text(summary: &Summary, lang: &str) -> String {
    parts(summary, lang)
        .iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(heading, items)| {
            let lines: Vec<String> = items
                .iter()
                .map(|i| match &i.at {
                    Some(at) => format!("- {} ({})", i.text, clock(at.ms)),
                    None => format!("- {}", i.text),
                })
                .collect();
            format!("{heading}\n{}", lines.join("\n"))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The clip a moment was said in, when its audio is on this device.
fn playback_at(
    db: &Database,
    note_id: &str,
    moment: &Moment,
) -> Option<Playback> {
    let clip = db
        .list_audios(note_id)
        .ok()?
        .into_iter()
        .find(|a| a.id == moment.audio_id)?;
    let path = audio::resolve_audio_path(&clip.file_path);
    if !std::path::Path::new(&path).exists() {
        return None;
    }
    Some(Playback {
        transcript: db.words_for_audio(&clip.id)?,
        audio_id: clip.id,
        path,
        duration_secs: clip.duration_secs.unwrap_or(0.0),
    })
}

/// "Résumer la réunion" under the tags of a recorded note, then the summary
/// kept with it, folded to two lines.
#[component]
pub fn SummaryCard(
    local_note_id: Signal<String>,
    content: Signal<String>,
    audios_version: Signal<u32>,
    player: NotePlayer,
) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut version = use_signal(|| 0u32);
    let mut busy = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut open = use_signal(|| false);
    let mut menu = use_signal(|| false);
    let mut copied = use_signal(|| false);

    let recorded = use_memo(move || {
        let id = local_note_id();
        let _ = audios_version();
        !id.is_empty() && db().has_any_audio(&id)
    });
    // A summary made or removed on another device arrives with the notes.
    let summary = use_memo(move || {
        let id = local_note_id();
        let _ = version();
        let _ = (app.notes_version)();
        (!id.is_empty()).then(|| db().summary_for_note(&id)).flatten()
    });

    let summarize = use_callback(move |_: ()| {
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        let id = local_note_id();
        let body = content();
        let lang = (app.current_lang)();
        spawn(async move {
            match summarize_note(&db(), &id, &body, &lang).await {
                Ok(_) => version += 1,
                Err(e) => error.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    });

    if !recorded() || (summary.read().is_none() && content().trim().is_empty())
    {
        return rsx! {};
    }

    let error_box = rsx! {
        if let Some(e) = error() {
            div { class: "mb-2 px-3 py-2 bg-ios-red/10 rounded-lg text-xs text-stone-600", "{e}" }
        }
    };

    let Some(kept) = summary().filter(|_| !busy()) else {
        let label = t(&lang, if busy() { "summary-busy" } else { "summary-action" });
        return rsx! {
            button {
                class: "pressable w-full h-11 mt-1 mb-2 flex items-center gap-2.5 px-4 rounded-full bg-warm-white border border-stone-200 shadow-card text-left",
                disabled: busy(),
                onclick: move |_| summarize.call(()),
                span {
                    class: "flex-1 text-sm font-medium text-stone-900",
                    style: if busy() { "animation: pulseSoft 1.2s ease-in-out infinite;" } else { "" },
                    "{label}"
                }
                span { class: "text-stone-400 flex items-center", IconCaretRight { size: 14 } }
            }
            {error_box}
        };
    };

    let stale = kept.source_hash != source_hash(&content());
    let folded: String = kept
        .body
        .key_points
        .iter()
        .chain(&kept.body.decisions)
        .map(|i| i.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let copy_text = as_text(&kept.body, &lang);

    rsx! {
        div { class: "relative mt-1 mb-2 rounded-xl border border-stone-200 bg-warm-white p-3",
            div { class: "flex items-center gap-1",
                button {
                    class: "pressable flex-1 min-h-[32px] flex items-center gap-2 text-left",
                    onclick: move |_| open.set(!open()),
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-stone-500",
                        {t(&lang, "summary-label")}
                    }
                    span {
                        class: "inline-block w-1.5 h-1.5 border-r-[1.5px] border-b-[1.5px] border-stone-400 chevron-pivot",
                        class: if open() { "-rotate-[135deg]" } else { "rotate-45" },
                    }
                }
                button {
                    class: "glass-disc relative w-8 h-8 -mr-1 flex items-center justify-center rounded-full",
                    class: if copied() { "text-ios-green" } else { "text-stone-500" },
                    aria_label: t(&lang, "summary-options"),
                    onclick: move |_| menu.set(!menu()),
                    if copied() {
                        IconCheck { size: 16 }
                    } else {
                        IconDotsThree { size: 18 }
                    }
                }
            }
            if menu() {
                div { class: "fixed inset-0 z-40", onclick: move |_| menu.set(false) }
                div { class: "absolute right-2 top-11 {kit::MENU_PANEL}",
                    button {
                        class: kit::MENU_ITEM,
                        onclick: move |_| {
                            menu.set(false);
                            summarize.call(());
                        },
                        IconArrowsClockwise { size: 16 }
                        {t(&lang, "summary-redo")}
                    }
                    button {
                        class: kit::MENU_ITEM,
                        onclick: move |_| {
                            menu.set(false);
                            crate::ui::clipboard::copy_text(&copy_text);
                            haptic("light");
                            copied.set(true);
                            spawn(async move {
                                futures_timer::Delay::new(std::time::Duration::from_millis(COPIED_MS)).await;
                                copied.set(false);
                            });
                        },
                        IconCopy { size: 16 }
                        {t(&lang, "summary-copy")}
                    }
                    div { class: kit::MENU_SEP }
                    button {
                        class: kit::MENU_ITEM_DANGER,
                        onclick: move |_| {
                            menu.set(false);
                            match db().delete_summary(&local_note_id()) {
                                Ok(()) => {
                                    open.set(false);
                                    version += 1;
                                }
                                Err(e) => error.set(Some(e)),
                            }
                        },
                        IconTrash { size: 16 }
                        {t(&lang, "summary-delete")}
                    }
                }
            }
            if stale {
                button {
                    class: "pressable mt-1 mb-1 w-full flex items-center gap-2 px-2.5 py-2 rounded-lg bg-ios-orange/10 text-left text-xs text-ios-orange-dark",
                    onclick: move |_| summarize.call(()),
                    span { class: "flex-1", {t(&lang, "summary-stale")} }
                    span { class: "font-semibold", {t(&lang, "summary-redo-short")} }
                }
            }
            if open() {
                div { class: "mt-2.5",
                    for (heading, items) in parts(&kept.body, &lang).into_iter().filter(|(_, items)| !items.is_empty()) {
                        div { key: "{heading}", class: "mt-3 first:mt-0",
                            p { class: PART_LABEL, "{heading}" }
                            ul { class: "space-y-1.5",
                                for (i, item) in items.iter().enumerate() {
                                    li { key: "{i}", class: "flex gap-2 text-sm leading-snug text-stone-800",
                                        span { class: "mt-[7px] w-1 h-1 rounded-full bg-stone-400 shrink-0" }
                                        span { class: "flex-1",
                                            "{item.text}"
                                            if let Some(at) = item.at.clone() {
                                                " "
                                                button {
                                                    class: "text-xs tabular-nums font-medium text-ios-orange-dark",
                                                    onclick: move |_| {
                                                        let Some(clip) = playback_at(&db(), &local_note_id(), &at) else {
                                                            return;
                                                        };
                                                        haptic("light");
                                                        let mut player = player;
                                                        player.play(clip, f64::from(at.ms) / 1000.0);
                                                    },
                                                    {clock(at.ms)}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                p { class: "mt-1.5 text-sm leading-snug text-stone-800 line-clamp-2", "{folded}" }
                button {
                    class: "mt-1 text-xs font-medium text-ios-orange-dark",
                    onclick: move |_| open.set(true),
                    {t(&lang, "summary-see-all")}
                }
            }
        }
        {error_box}
    }
}
