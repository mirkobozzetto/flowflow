use flowflow::ui::hooks::note_scrubber::{build_note_scrubber, ScrubberLabels};

const LABELS: ScrubberLabels = ScrubberLabels {
    chapter: "Chapitre",
    start: "Début de la note",
    end: "Fin \"de\" la note",
};

#[test]
fn build_note_scrubber_fills_ids_and_labels() {
    let s = build_note_scrubber("note-scrubber", "reading-bar", &LABELS);
    for placeholder in
        ["__ROOT__", "__BAR__", "__CHAPTER__", "__START__", "__END__"]
    {
        assert!(!s.contains(placeholder), "leftover {placeholder}");
    }
    assert!(s.contains("\"note-scrubber\""));
    assert!(s.contains("\"reading-bar\""));
    assert!(s.contains("\"Début de la note\""));
    assert!(
        s.contains(r#""Fin \"de\" la note""#),
        "a label is a JS string literal, quotes escaped"
    );
}

#[test]
fn note_scrubber_script_reads_the_note() {
    let s = build_note_scrubber("note-scrubber", "reading-bar", &LABELS);
    assert!(s.contains(".note-scroll"), "the note's scroller is driven");
    assert!(s.contains("[data-chapter]"), "chapters mark the track");
    assert!(s.contains("dioxus.send"), "a grab reaches Rust");
    assert!(s.contains("dioxus.recv"), "the eval stays alive");
}
