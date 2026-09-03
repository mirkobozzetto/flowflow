use crate::application::i18n::{t, t_args};
use crate::domain::{Note, NoteType, Thread};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::notes::note_card::NoteCard;
use crate::ui::notes::search_filters::SearchFilters;
use crate::ui::thread::ThreadCard;
use crate::ui::{AppState, NoteFilters, RowMenu};
use dioxus::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

const NOTES_PAGE: usize = 30;

#[derive(Clone, PartialEq)]
enum FeedItem {
    Note(Note),
    Thread(Thread),
}

impl FeedItem {
    fn recency(&self) -> &str {
        match self {
            FeedItem::Note(n) => &n.created_at,
            FeedItem::Thread(t) => &t.modified_at,
        }
    }
}

fn note_matches(n: &Note, q: &str) -> bool {
    q.is_empty()
        || n.title.as_deref().unwrap_or("").to_lowercase().contains(q)
        || n.content.to_lowercase().contains(q)
        || n.tags.iter().any(|t| t.to_lowercase().contains(q))
}

fn note_passes(
    n: &Note,
    f: NoteFilters,
    with_reminder: &HashSet<String>,
    with_document: &HashSet<String>,
) -> bool {
    !f.thread
        && (!f.voice || n.note_type == NoteType::Voice)
        && (!f.reminder || with_reminder.contains(&n.id))
        && (!f.document || with_document.contains(&n.id))
}

/// The placeholder names the active filter, so an icon-only toggle never has to be
/// guessed. Empty means no single-filter wording fits and the count is used.
fn filter_placeholder_key(f: NoteFilters) -> &'static str {
    match (f.voice, f.reminder, f.document, f.thread) {
        (true, false, false, false) => "note-filter-voice",
        (false, true, false, false) => "note-filter-reminder",
        (false, false, true, false) => "note-filter-document",
        (false, false, false, true) => "note-filter-thread",
        _ => "",
    }
}

#[component]
pub fn NotesList() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut visible_count = use_signal(|| NOTES_PAGE);

    let notes = use_memo(move || {
        let _v = (app.notes_version)();
        let _s = (app.sync_data_version)();
        let db = db();
        let q = (app.search_query)().to_lowercase();
        let f = (app.note_filters)();
        let (root_notes, feed_threads) = match (app.selected_folder_id)() {
            Some(fid) => (
                db.list_root_notes_in_folder(&fid).unwrap_or_default(),
                db.list_feed_threads_in_folder(&fid).unwrap_or_default(),
            ),
            None => (
                db.list_root_notes().unwrap_or_default(),
                db.list_feed_threads().unwrap_or_default(),
            ),
        };
        let with_reminder = if f.reminder {
            db.note_ids_with_active_reminder().unwrap_or_default()
        } else {
            HashSet::new()
        };
        let with_document = if f.document {
            db.note_ids_with_attachment().unwrap_or_default()
        } else {
            HashSet::new()
        };
        let mut items: Vec<FeedItem> = root_notes
            .into_iter()
            .filter(|n| {
                note_matches(n, &q)
                    && note_passes(n, f, &with_reminder, &with_document)
            })
            .map(FeedItem::Note)
            .collect();
        // ponytail: a thread only ever answers its own toggle. Matching one on
        // "a member carries a reminder" means loading every thread's notes, one
        // query per thread; do it only once the feed actually asks for that.
        let keep_threads = f.is_empty() || f.thread;
        for th in feed_threads {
            if keep_threads
                && (q.is_empty() || th.title.to_lowercase().contains(&q))
            {
                items.push(FeedItem::Thread(th));
            }
        }
        items.sort_by(|a, b| b.recency().cmp(a.recency()));
        items
    });

    use_effect(move || {
        let _ = (app.search_query)();
        let _ = (app.selected_folder_id)();
        let _ = (app.note_filters)();
        visible_count.set(NOTES_PAGE);
    });

    use_future(move || async move {
        let mut eval = dioxus::document::eval(
            r#"
            function nearBottom() {
                var sc = document.getElementById('notes-scroll');
                if (!sc) return false;
                return sc.scrollTop + sc.clientHeight >= sc.scrollHeight - 800;
            }
            var ticking = false;
            function onScroll() {
                if (ticking) return;
                ticking = true;
                requestAnimationFrame(function() {
                    ticking = false;
                    if (nearBottom()) { dioxus.send('more'); }
                });
            }
            setInterval(function() {
                var sc = document.getElementById('notes-scroll');
                if (sc && !sc._pagerBound) {
                    sc._pagerBound = true;
                    sc.addEventListener('scroll', onScroll);
                }
                if (nearBottom()) { dioxus.send('more'); }
            }, 600);
            "#,
        );
        while let Ok(msg) = eval.recv::<String>().await {
            if msg == "more" {
                let total = notes.peek().len();
                let cur = *visible_count.peek();
                if cur < total {
                    visible_count.set((cur + NOTES_PAGE).min(total));
                }
            }
        }
    });

    let has_query = !(app.search_query)().is_empty();
    let lang = (app.current_lang)();
    let filters = (app.note_filters)();
    let placeholder = if filters.is_empty() {
        t(&lang, "note-list-search-placeholder")
    } else {
        match filter_placeholder_key(filters) {
            "" => t_args(
                &lang,
                "note-filter-many",
                &[("count", &filters.count().to_string())],
            ),
            key => t(&lang, key),
        }
    };
    let narrowed = has_query || !filters.is_empty();

    // A read-only theme inside a shared space says so here, next to its notes,
    // rather than letting the user write one and lose it to a refusal.
    let read_only = use_memo(move || {
        let _v = (app.folders_version)();
        matches!(
            (app.selected_folder_id)().map(|fid| {
                crate::application::space::folder_right(&db(), &fid)
            }),
            Some(crate::application::space::FolderRight::SpaceReadOnly)
        )
    });

    rsx! {
        if read_only() {
            div { class: "mb-3 px-3 py-2 rounded-xl bg-stone-100 text-xs text-stone-500",
                {t(&lang, "space-cannot-write")}
            }
        }
        div { class: "mb-3",
            div { class: "relative",
                div { class: "absolute left-3 top-1/2 -translate-y-1/2 text-stone-400",
                    IconMagnifyingGlass { size: 16 }
                }
                input {
                    id: "note-search",
                    class: "w-full bg-warm-white border border-stone-200 rounded-xl pl-9 py-2.5 text-sm outline-none text-stone-900 placeholder-stone-400 hover:border-stone-300 focus:border-ios-orange-dark focus:ring-[3px] focus:ring-ios-orange-50 transition-colors duration-150",
                    class: if has_query { "pr-[9.5rem]" } else { "pr-32" },
                    placeholder,
                    value: "{app.search_query}",
                    oninput: move |evt| app.search_query.set(evt.value()),
                }
                if has_query {
                    button {
                        class: "absolute right-32 top-1/2 -translate-y-1/2 text-stone-400 active:text-stone-600 hover:text-stone-600 p-1 transition-colors duration-150",
                        onclick: move |_| app.search_query.set(String::new()),
                        IconX { size: 14 }
                    }
                }
                SearchFilters {}
            }
        }
        if notes().is_empty() {
            div { class: "flex-1 flex flex-col items-center justify-center gap-2 h-[60vh]",
                if narrowed {
                    p { class: "text-lg text-stone-400", {t(&lang, "note-list-no-results")} }
                    p { class: "text-sm text-stone-400", {t(&lang, "note-list-no-results-hint")} }
                } else {
                    img { src: asset!("/assets/flowflow-icon-300.png"), width: "120", height: "120", class: "mb-6 object-contain" }
                    p { class: "text-lg font-semibold text-stone-900", {t(&lang, "note-list-welcome")} }
                    p { class: "text-sm text-stone-400 mt-1", {t(&lang, "note-list-first-note-hint")} }
                }
            }
        } else {
            // Dims the rest of the list while a note menu is open, so the pressed
            // card reads as picked up. Visual only: the outside-click catcher is
            // the global row-menu backdrop.
            if matches!((app.row_menu)(), Some(RowMenu::Note(_))) {
                div { class: "absolute inset-0 z-10 bg-stone-900/20 pointer-events-none backdrop-fade" }
            }
            div { class: "safe-pb-32 lg:grid lg:grid-cols-2 lg:gap-2.5",
                for item in notes().into_iter().take(visible_count()) {
                    match item {
                        FeedItem::Note(note) => rsx! { NoteCard { key: "{note.id}", note: note } },
                        FeedItem::Thread(thread) => rsx! { ThreadCard { key: "{thread.id}", thread: thread } },
                    }
                }
            }
        }
    }
}
