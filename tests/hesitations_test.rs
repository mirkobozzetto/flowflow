use flowflow::domain::{Transcript, Word};
use flowflow::infrastructure::transcription::{
    clean_hesitations, clean_hesitations_words,
};

/// Words 400 ms apart, so a survivor's timing is visibly its own.
fn timed(texts: &[&str]) -> Vec<Word> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let start = i as u32 * 400;
            Word::new(*t, start, start + 400, 0.9)
        })
        .collect()
}

#[test]
fn test_empty_input() {
    assert_eq!(clean_hesitations(""), "");
}

#[test]
fn test_no_hesitations() {
    let text = "Bonjour, je suis un développeur freelance.";
    assert_eq!(clean_hesitations(text), text);
}

#[test]
fn test_french_isolated_euh() {
    let input = "Je voulais euh dire que c'est important.";
    assert_eq!(
        clean_hesitations(input),
        "Je voulais dire que c'est important."
    );
}

#[test]
fn test_french_with_extra_letters() {
    let input = "Alors euuuuh on commence.";
    assert_eq!(clean_hesitations(input), "Alors on commence.");
}

#[test]
fn test_hmm_at_start() {
    let input = "Hmm, je pense que oui.";
    assert_eq!(clean_hesitations(input), "je pense que oui.");
}

#[test]
fn test_english_um() {
    let input = "I think um we should go.";
    assert_eq!(clean_hesitations(input), "I think we should go.");
}

#[test]
fn test_english_you_know() {
    let input = "It's hard, you know, to explain.";
    assert_eq!(clean_hesitations(input), "It's hard, to explain.");
}

#[test]
fn test_multiple_hesitations() {
    let input = "Euh, ben, je sais pas hein.";
    assert_eq!(clean_hesitations(input), "je sais pas.");
}

#[test]
fn test_preserve_real_words() {
    let input = "Eric et Aurélie travaillent ensemble.";
    assert_eq!(clean_hesitations(input), input);
}

#[test]
fn test_pfff() {
    let input = "Pfff, c'est compliqué.";
    assert_eq!(clean_hesitations(input), "c'est compliqué.");
}

#[test]
fn test_mouais() {
    let input = "Mouais, on verra bien.";
    assert_eq!(clean_hesitations(input), "on verra bien.");
}

#[test]
fn test_collapse_whitespace() {
    let input = "Mot   euh    final.";
    assert_eq!(clean_hesitations(input), "Mot final.");
}

#[test]
fn survivors_keep_their_own_timings() {
    let out = clean_hesitations_words(timed(&["Je", "voulais", "euh", "dire"]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["Je", "voulais", "dire"]
    );
    assert_eq!((out[2].start_ms, out[2].end_ms), (1200, 1600));
}

/// The failure the string-level fixture cannot catch: "hein." only matches once
/// its full stop is set aside, and that full stop has to land on "pas".
#[test]
fn a_hesitation_carrying_punctuation_is_dropped_and_gives_its_punctuation_back()
{
    let out = clean_hesitations_words(timed(&["je", "sais", "pas", "hein."]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["je", "sais", "pas."]
    );
    assert_eq!((out[2].start_ms, out[2].end_ms), (800, 1200));
}

#[test]
fn a_two_word_hesitation_is_dropped_as_one_unit() {
    let out = clean_hesitations_words(timed(&[
        "It's", "hard,", "you", "know,", "to", "explain.",
    ]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["It's", "hard,", "to", "explain."]
    );
}

#[test]
fn a_leading_hesitation_takes_its_punctuation_with_it() {
    let out = clean_hesitations_words(timed(&["Hmm,", "je", "pense"]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["je", "pense"]
    );
    assert_eq!(out[0].start_ms, 400);
}

#[test]
fn cleaning_preserves_the_stored_text_invariant() {
    let out = clean_hesitations_words(timed(&[
        "Euh,", "ben,", "je", "sais", "hein.",
    ]));
    let transcript = Transcript::new(out);
    assert_eq!(
        transcript.text().split_whitespace().count(),
        transcript.words.len()
    );
    assert_eq!(transcript.text(), "je sais.");
}
