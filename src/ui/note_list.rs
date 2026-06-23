use crate::application::i18n::t;
use crate::domain::{Note, Thread};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::note_card::NoteCard;
use crate::ui::thread::ThreadCard;
use crate::ui::AppState;
use dioxus::prelude::*;
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
        let mut items: Vec<FeedItem> = root_notes
            .into_iter()
            .filter(|n| note_matches(n, &q))
            .map(FeedItem::Note)
            .collect();
        for th in feed_threads {
            if q.is_empty() || th.title.to_lowercase().contains(&q) {
                items.push(FeedItem::Thread(th));
            }
        }
        items.sort_by(|a, b| b.recency().cmp(a.recency()));
        items
    });

    use_effect(move || {
        let _ = (app.search_query)();
        let _ = (app.selected_folder_id)();
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

    rsx! {
        div { class: "mb-3",
            div { class: "relative",
                div { class: "absolute left-3 top-1/2 -translate-y-1/2 text-stone-400",
                    IconMagnifyingGlass { size: 16 }
                }
                input {
                    id: "note-search",
                    class: "w-full bg-warm-white border border-stone-200 rounded-xl pl-9 pr-9 py-2.5 text-sm outline-none text-stone-900 placeholder-stone-400 hover:border-stone-300 focus:border-ios-orange-dark focus:ring-[3px] focus:ring-ios-orange-50 transition-colors duration-150",
                    placeholder: t(&lang, "note-list-search-placeholder"),
                    value: "{app.search_query}",
                    oninput: move |evt| app.search_query.set(evt.value()),
                }
                if has_query {
                    button {
                        class: "absolute right-2 top-1/2 -translate-y-1/2 text-stone-400 active:text-stone-600 hover:text-stone-600 p-1 transition-colors duration-150",
                        onclick: move |_| app.search_query.set(String::new()),
                        IconX { size: 14 }
                    }
                }
            }
        }
        if notes().is_empty() {
            div { class: "flex-1 flex flex-col items-center justify-center gap-2 h-[60vh]",
                if has_query {
                    p { class: "text-lg text-stone-400", {t(&lang, "note-list-no-results")} }
                    p { class: "text-sm text-stone-400", {t(&lang, "note-list-no-results-hint")} }
                } else {
                    img { src: asset!("/assets/flowflow-icon-300.png"), width: "200", height: "200", class: "mb-6 rounded-2xl" }
                    p { class: "text-lg font-semibold text-stone-900", {t(&lang, "note-list-welcome")} }
                    p { class: "text-sm text-stone-400 mt-1", {t(&lang, "note-list-first-note-hint")} }
                }
            }
        } else {
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
