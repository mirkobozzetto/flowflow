use flowflow::infrastructure::transcription::whisper::mean_text_probability;

/// Whisper's own end-of-transcript id on the multilingual vocabularies. Only its
/// role as the boundary matters here: at or above it, a token is special or a
/// timestamp.
const FIRST_SPECIAL: i32 = 50_257;

/// R8. Under `max_len(1)` a segment holds one word, so special and timestamp
/// tokens are a large share of every segment's token set. Averaging over them
/// would report a confidence that describes nothing.
#[test]
fn special_and_timestamp_tokens_are_excluded_from_the_confidence() {
    let tokens = [
        (FIRST_SPECIAL + 100, 0.10), // a timestamp token
        (1_234, 0.90),               // real text
        (5_678, 0.80),               // real text
        (FIRST_SPECIAL, 0.05),       // end of transcript
    ];
    assert!(
        (mean_text_probability(&tokens, FIRST_SPECIAL) - 0.85).abs() < 1e-6
    );
}

#[test]
fn a_segment_of_only_special_tokens_reports_no_confidence() {
    let tokens = [(FIRST_SPECIAL, 0.9), (FIRST_SPECIAL + 1, 0.9)];
    assert_eq!(mean_text_probability(&tokens, FIRST_SPECIAL), 0.0);
}

#[test]
fn an_empty_segment_reports_no_confidence() {
    assert_eq!(mean_text_probability(&[], FIRST_SPECIAL), 0.0);
}

#[test]
fn a_segment_of_only_text_averages_every_token() {
    let tokens = [(10, 1.0), (20, 0.5)];
    assert!(
        (mean_text_probability(&tokens, FIRST_SPECIAL) - 0.75).abs() < 1e-6
    );
}
