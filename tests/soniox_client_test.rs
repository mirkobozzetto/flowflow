use flowflow::domain::{Dictionary, Transcript};
use flowflow::infrastructure::transcription::client::{
    group_tokens_to_words, transcription_request_body, SonioxToken,
};

/// A real French response captured from the async transcript endpoint on
/// 2026-07-26. It is the evidence behind the word-boundary rule, not a mock.
const FIXTURE: &str = include_str!("fixtures/soniox_tokens.json");

fn fixture() -> (String, Vec<SonioxToken>) {
    let parsed: serde_json::Value =
        serde_json::from_str(FIXTURE).expect("fixture parses");
    let text = parsed["text"].as_str().expect("text").to_string();
    let tokens: Vec<SonioxToken> =
        serde_json::from_value(parsed["tokens"].clone()).expect("tokens");
    (text, tokens)
}

#[test]
fn body_carries_the_dictionary_terms_in_the_native_context_field() {
    let dict = Dictionary::parse("Sony Ox\tSoniox\nDioxus");
    let body = transcription_request_body("file-1", Some("fr"), &dict.terms());

    assert_eq!(
        body["context"]["terms"],
        serde_json::json!(["Soniox", "Dioxus"])
    );
    assert_eq!(body["language_hints"], serde_json::json!(["fr"]));
    assert_eq!(body["model"], serde_json::json!("stt-async-v4"));
}

#[test]
fn an_empty_dictionary_leaves_the_request_untouched() {
    let empty = Dictionary::default();
    let body = transcription_request_body("file-1", Some("fr"), &empty.terms());

    assert!(body.get("context").is_none());
    assert_eq!(body, transcription_request_body("file-1", Some("fr"), &[]));
}

/// OQ1: a leading space opens a new word, anything else continues the current
/// one. The check that makes it more than an assumption is that the grouped
/// words rebuild Soniox's own `text` field exactly.
#[test]
fn grouping_sub_word_tokens_reproduces_the_official_text() {
    let (text, tokens) = fixture();
    let words = group_tokens_to_words(&tokens);
    assert_eq!(Transcript::new(words).text(), text.trim());
}

#[test]
fn sub_word_tokens_merge_into_one_word_spanning_their_timings() {
    let (_, tokens) = fixture();
    let words = group_tokens_to_words(&tokens);

    // "Bon" + "j" + "our" + "." arrive as four tokens and are one spoken word.
    assert_eq!(words[0].text, "Bonjour.");
    assert_eq!(words[0].start_ms, tokens[0].start_ms);
    assert_eq!(words[0].end_ms, tokens[3].end_ms);
    assert!(words[0].confidence > 0.5 && words[0].confidence <= 1.0);
}

#[test]
fn punctuation_never_becomes_a_word_of_its_own() {
    let (_, tokens) = fixture();
    let words = group_tokens_to_words(&tokens);
    assert!(!words.is_empty());
    for word in &words {
        assert!(!word.trimmed().is_empty(), "{:?} is punctuation only", word);
        assert!(!word.text.chars().any(char::is_whitespace));
        assert!(word.end_ms >= word.start_ms);
    }
}

#[test]
fn a_response_without_tokens_yields_no_words_rather_than_an_error() {
    assert!(group_tokens_to_words(&[]).is_empty());
}

/// The word array has to survive the cleaners intact: cleaning is what the
/// stored text is derived from, so a mismatch here is a mismatch on screen.
#[test]
fn the_cleaned_word_count_still_matches_the_cleaned_text() {
    let (_, tokens) = fixture();
    let dict = Dictionary::parse("LensDB\tLanceDB");
    let words =
        flowflow::infrastructure::transcription::clean_hesitations_words(
            group_tokens_to_words(&tokens),
        );
    let transcript = Transcript::new(dict.apply_words(words));

    assert_eq!(
        transcript.text().split_whitespace().count(),
        transcript.words.len()
    );
    assert!(
        transcript.text().contains("LanceDB"),
        "declared term corrected: {}",
        transcript.text()
    );
    assert!(
        !transcript.text().contains("euh"),
        "hesitation dropped: {}",
        transcript.text()
    );
}
