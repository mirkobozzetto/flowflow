use dioxus::document::Eval;
use dioxus::prelude::*;

const CHAPTER_SCROLL_JS: &str = include_str!("chapter_scroll.js");

/// The chapter under the reading bar, and a way to scroll to one.
#[derive(Clone, Copy)]
pub struct ChapterScroll {
    pub current: Signal<usize>,
    eval: Signal<Option<Eval>>,
}

impl ChapterScroll {
    /// Scroll so chapter `n` sits under the bar; `None` goes to the note's top.
    pub fn jump(&self, n: Option<usize>) {
        if let Some(eval) = self.eval.peek().as_ref() {
            let _ = eval.send(n.map_or(-1, |n| n as i64));
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
    use_future(move || async move {
        let mut channel = dioxus::document::eval(&build_chapter_scroll(bar_id));
        eval.set(Some(channel.clone()));
        while let Ok(n) = channel.recv::<usize>().await {
            current.set(n);
        }
    });
    ChapterScroll { current, eval }
}
