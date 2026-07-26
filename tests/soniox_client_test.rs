use flowflow::domain::Dictionary;
use flowflow::infrastructure::transcription::client::transcription_request_body;

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
