use flowflow::domain::reading::{
    locate_paragraphs, paragraph_ranges, Segment, PAUSE_PARAGRAPH_MS,
};
use flowflow::domain::{Transcript, Word};

/// Words spoken back to back, one every 300 ms, with a pause of `pause_ms`
/// before each index listed in `pauses_before`.
fn timed(text: &str, pauses_before: &[usize], pause_ms: u32) -> Vec<Word> {
    let mut at = 0u32;
    text.split_whitespace()
        .enumerate()
        .map(|(i, token)| {
            if pauses_before.contains(&i) {
                at += pause_ms;
            }
            let word = Word::new(token, at, at + 250, 1.0);
            at += 300;
            word
        })
        .collect()
}

fn paragraph(audio: usize, words: std::ops::Range<usize>) -> Segment {
    Segment::Paragraph { audio, words }
}

#[test]
fn a_pause_of_two_seconds_starts_a_paragraph() {
    let words = timed("one two three four five", &[2, 4], 2000);
    assert_eq!(
        paragraph_ranges(&words, PAUSE_PARAGRAPH_MS),
        vec![0..2, 2..4, 4..5]
    );
}

#[test]
fn a_shorter_pause_keeps_the_paragraph() {
    let words = timed("one two three", &[1], 1900);
    assert_eq!(paragraph_ranges(&words, PAUSE_PARAGRAPH_MS), vec![0..3]);
    assert!(paragraph_ranges(&[], PAUSE_PARAGRAPH_MS).is_empty());
}

#[test]
fn each_transcript_is_found_in_the_body_with_typed_text_around() {
    let first = timed(
        "we start the meeting now with the whole team. Yes.",
        &[9],
        3000,
    );
    let second = timed("second clip here", &[], 0);
    let body = format!(
        "## Agenda\n{}\n{}\nmy own conclusion",
        Transcript::new(first.clone()).text(),
        Transcript::new(second.clone()).text()
    );

    let segments =
        locate_paragraphs(&body, &[&first, &second], PAUSE_PARAGRAPH_MS);

    assert_eq!(
        segments,
        vec![
            Segment::Text("## Agenda".into()),
            paragraph(0, 0..9),
            paragraph(0, 9..10),
            paragraph(1, 0..3),
            Segment::Text("my own conclusion".into()),
        ]
    );
}

#[test]
fn an_edited_paragraph_falls_back_to_text_and_the_others_keep_their_timings() {
    let words = timed(
        "first paragraph is here. second paragraph has a typo inside it \
         right now. third paragraph closes it with a few more words",
        &[4, 13],
        2500,
    );
    let body = Transcript::new(words.clone())
        .text()
        .replace("a typo", "no typo");

    let segments = locate_paragraphs(&body, &[&words], PAUSE_PARAGRAPH_MS);

    assert_eq!(
        segments,
        vec![
            paragraph(0, 0..4),
            Segment::Text(
                "second paragraph has no typo inside it right now.".into()
            ),
            paragraph(0, 13..22),
        ]
    );
}

#[test]
fn a_short_paragraph_is_never_matched_further_down() {
    // "Yeah." was deleted by hand; the "Yeah." spoken later must not be taken
    // for it, or every paragraph in between would lose its timings.
    let words = timed(
        "Yeah. so we talk about the plan for the quarter today. Yeah.",
        &[1, 11],
        2500,
    );
    let body = Transcript::new(words.clone())
        .text()
        .replacen("Yeah. ", "", 1);

    let segments = locate_paragraphs(&body, &[&words], PAUSE_PARAGRAPH_MS);

    assert_eq!(segments, vec![paragraph(0, 1..11), paragraph(0, 11..12)]);
}

#[test]
fn a_match_inside_a_longer_word_is_rejected() {
    let words = timed("plan", &[], 0);
    let segments =
        locate_paragraphs("planning ahead", &[&words], PAUSE_PARAGRAPH_MS);
    assert_eq!(segments, vec![Segment::Text("planning ahead".into())]);
}
