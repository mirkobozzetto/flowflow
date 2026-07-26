use flowflow::domain::transcript::{apply_rewrites, Rewrite};
use flowflow::domain::{Transcript, Word};

fn timed(text: &str, start_ms: u32, end_ms: u32) -> Word {
    Word::new(text, start_ms, end_ms, 0.9)
}

/// The R1 gate: the stored text and the rendered spans can never disagree,
/// because the text is a join of the very words that get rendered.
#[test]
fn word_count_always_matches_the_token_count_of_the_stored_text() {
    let cases = [
        "",
        "seul",
        "Bonjour, je suis un développeur freelance.",
        "Ponctuation lourde !!! Vraiment ?? Oui... voilà.",
        "  espaces   multiples   partout  ",
        "l'anceDB, la base vectorielle",
        "Un tiret - isolé et des «guillemets»",
    ];
    for case in cases {
        let transcript = Transcript::from_text(case);
        assert_eq!(
            transcript.text().split_whitespace().count(),
            transcript.words.len(),
            "invariant broken on {case:?}"
        );
    }
}

#[test]
fn no_word_is_punctuation_only_or_carries_whitespace() {
    let transcript =
        Transcript::from_text("Alors , voilà . Un test ! Vraiment ?");
    for word in &transcript.words {
        assert!(!word.text.is_empty());
        assert!(
            !word.text.chars().any(char::is_whitespace),
            "{:?} carries whitespace",
            word.text
        );
        assert!(
            !word.trimmed().is_empty(),
            "{:?} is punctuation only",
            word.text
        );
    }
    assert_eq!(transcript.text(), "Alors, voilà. Un test! Vraiment?");
}

#[test]
fn text_to_words_and_back_round_trips_normalized_text() {
    let text = "Une phrase simple, avec de la ponctuation.";
    assert_eq!(Transcript::from_text(text).text(), text);
}

#[test]
fn trimmed_and_trailing_punctuation_split_a_word() {
    let word = Word::untimed("euh,");
    assert_eq!(word.trimmed(), "euh");
    assert_eq!(word.trailing_punctuation(), ",");

    let clean = Word::untimed("voilà");
    assert_eq!(clean.trimmed(), "voilà");
    assert_eq!(clean.trailing_punctuation(), "");
}

#[test]
fn word_index_at_ms_tracks_playback() {
    let transcript = Transcript::new(vec![
        timed("Bonjour", 0, 500),
        timed("le", 500, 700),
        timed("monde", 700, 1200),
    ]);
    assert_eq!(transcript.word_index_at_ms(0), Some(0));
    assert_eq!(transcript.word_index_at_ms(499), Some(0));
    assert_eq!(transcript.word_index_at_ms(500), Some(1));
    assert_eq!(transcript.word_index_at_ms(900), Some(2));
    // Past the end of the last word: nothing is being spoken any more.
    assert_eq!(transcript.word_index_at_ms(5_000), None);
}

#[test]
fn word_index_holds_through_the_silence_between_two_words() {
    let transcript =
        Transcript::new(vec![timed("un", 0, 200), timed("deux", 900, 1100)]);
    assert_eq!(transcript.word_index_at_ms(500), Some(0));
}

#[test]
fn empty_and_untimed_transcripts_are_recognisable() {
    assert!(Transcript::default().is_empty());
    assert!(!Transcript::from_text("du texte").has_timings());
    assert!(Transcript::new(vec![timed("mot", 0, 400)]).has_timings());
}

#[test]
fn a_rewrite_inside_one_word_keeps_that_words_timing() {
    let words = vec![timed("pareil", 0, 400), timed("l'anceDB,", 400, 1200)];
    // "l'anceDB" occupies bytes 7..15 of "pareil l'anceDB,".
    let out = apply_rewrites(
        words,
        &[Rewrite {
            start: 7,
            end: 15,
            text: "LanceDB".into(),
        }],
    );
    assert_eq!(out.len(), 2);
    assert_eq!(out[1].text, "LanceDB,");
    assert_eq!((out[1].start_ms, out[1].end_ms), (400, 1200));
}

#[test]
fn a_rewrite_spanning_two_words_collapses_them_into_one_span() {
    let words = vec![
        timed("que", 0, 200),
        timed("Sony", 200, 600),
        timed("Ox", 600, 1000),
        timed("coute", 1000, 1400),
    ];
    // "Sony Ox" occupies bytes 4..11 of "que Sony Ox coute".
    let out = apply_rewrites(
        words,
        &[Rewrite {
            start: 4,
            end: 11,
            text: "Soniox".into(),
        }],
    );
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["que", "Soniox", "coute"]
    );
    assert_eq!((out[1].start_ms, out[1].end_ms), (200, 1000));
}

#[test]
fn a_replacement_carrying_a_space_still_yields_whitespace_free_words() {
    let words = vec![timed("Mirko", 0, 400), timed("Bose,", 400, 1200)];
    let out = apply_rewrites(
        words,
        &[Rewrite {
            start: 0,
            end: 10,
            text: "Mirko Bozzetto".into(),
        }],
    );
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["Mirko", "Bozzetto,"]
    );
    assert_eq!(out[0].start_ms, 0);
    assert_eq!(out[1].end_ms, 1200);
    let rebuilt = Transcript::new(out);
    assert_eq!(
        rebuilt.text().split_whitespace().count(),
        rebuilt.words.len()
    );
}

#[test]
fn no_rewrites_leaves_the_words_untouched() {
    let words = vec![timed("intact", 0, 400)];
    assert_eq!(apply_rewrites(words.clone(), &[]), words);
}
