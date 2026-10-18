use flowflow::ui::hooks::chapter_scroll::build_chapter_scroll;

#[test]
fn build_chapter_scroll_fills_the_bar_id() {
    let s = build_chapter_scroll("reading-bar");
    assert!(
        !s.contains("__BAR__"),
        "leftover placeholder in generated script"
    );
    assert!(s.contains("barId: \"reading-bar\""));
}

#[test]
fn chapter_scroll_script_reports_and_jumps() {
    let s = build_chapter_scroll("reading-bar");
    assert!(
        s.contains("[data-chapter]"),
        "headings are found by data-chapter"
    );
    assert!(s.contains(".note-scroll"), "the note's scroller is tracked");
    assert!(
        s.contains("dioxus.send"),
        "the current chapter goes back to Rust"
    );
    assert!(s.contains("dioxus.recv"), "jumps come from Rust");
}
