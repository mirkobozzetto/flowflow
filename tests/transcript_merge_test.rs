use flowflow::domain::merge_transcript_into_body;

#[test]
fn empty_body_takes_the_transcript_alone() {
    assert_eq!(
        merge_transcript_into_body("", "bonjour"),
        Some("bonjour".to_string())
    );
}

#[test]
fn existing_body_gets_the_transcript_on_its_own_line() {
    assert_eq!(
        merge_transcript_into_body("déjà là", "suite"),
        Some("déjà là\nsuite".to_string())
    );
}

#[test]
fn an_empty_transcript_writes_nothing() {
    assert_eq!(merge_transcript_into_body("déjà là", ""), None);
    assert_eq!(merge_transcript_into_body("", ""), None);
}

#[test]
fn a_whitespace_only_transcript_writes_nothing() {
    assert_eq!(merge_transcript_into_body("déjà là", "   \n\t "), None);
    assert_eq!(merge_transcript_into_body("", "   \n\t "), None);
}

#[test]
fn the_transcript_is_trimmed() {
    assert_eq!(
        merge_transcript_into_body("", "  bonjour  "),
        Some("bonjour".to_string())
    );
}

#[test]
fn a_multi_line_transcript_is_appended_whole() {
    assert_eq!(
        merge_transcript_into_body("a", "b\nc"),
        Some("a\nb\nc".to_string())
    );
}
