use crate::infrastructure::platform::haptic;
use dioxus::document::Eval;
use dioxus::prelude::*;
use std::time::{Duration, Instant};

const CHAPTER_SCROLL_JS: &str = include_str!("chapter_scroll.js");
/// No tick this long after a jump: its smooth scroll crosses every chapter in
/// between, and the pick already gave its own tap.
const JUMP_QUIET: Duration = Duration::from_millis(900);

/// The chapter under the reading bar, and a way to scroll to one.
#[derive(Clone, Copy)]
pub struct ChapterScroll {
    pub current: Signal<usize>,
    eval: Signal<Option<Eval>>,
    jumped_at: Signal<Option<Instant>>,
}

impl ChapterScroll {
    /// Scroll so chapter `n` sits under the bar; `None` goes to the note's top.
    pub fn jump(&self, n: Option<usize>) {
        let mut jumped_at = self.jumped_at;
        jumped_at.set(Some(Instant::now()));
        self.send(n.map_or(-1, |n| n as i64));
    }

    /// Scroll the chapter list `list_id` to its `[data-current]` row.
    pub fn center(&self, list_id: &str) {
        self.send(list_id);
    }

    fn send(&self, msg: impl serde::Serialize) {
        if let Some(eval) = self.eval.peek().as_ref() {
            let _ = eval.send(msg);
        }
    }
}

// Kept apart from the hook (which needs a Dioxus runtime) so it is testable.
pub fn build_chapter_scroll(bar_id: &str) -> String {
    CHAPTER_SCROLL_JS.replace("__BAR__", bar_id)
}

pub fn use_chapter_scroll(bar_id: &'static str) -> ChapterScroll {
    let mut current = use_signal(|| 0usize);
    let mut eval: Signal<Option<Eval>> = use_signal(|| None);
    let jumped_at: Signal<Option<Instant>> = use_signal(|| None);
    use_future(move || async move {
        let mut channel = dioxus::document::eval(&build_chapter_scroll(bar_id));
        eval.set(Some(channel));
        // The first report only says where the note opened.
        let mut first = true;
        while let Ok(n) = channel.recv::<usize>().await {
            let quiet =
                jumped_at.peek().is_some_and(|t| t.elapsed() < JUMP_QUIET);
            if !first && !quiet && n != *current.peek() {
                haptic("selection");
            }
            first = false;
            current.set(n);
        }
    });
    ChapterScroll {
        current,
        eval,
        jumped_at,
    }
}
