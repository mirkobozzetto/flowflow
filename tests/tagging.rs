use flowflow::application::error::LlmError;
use flowflow::application::tagging::parse_tags;

#[test]
fn test_parse_tags_plain_array() {
    let response = r#"["tag1", "tag2", "tag3"]"#;
    let tags = parse_tags(response).unwrap();
    assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);
}

#[test]
fn test_parse_tags_with_whitespace() {
    let response = "  \n  [\"meeting\", \"project\"]  \n  ";
    let tags = parse_tags(response).unwrap();
    assert_eq!(tags, vec!["meeting", "project"]);
}

#[test]
fn test_parse_tags_wrapped_in_text() {
    let response = "Here are the tags: [\"alpha\", \"beta\"] hope this helps";
    let tags = parse_tags(response).unwrap();
    assert_eq!(tags, vec!["alpha", "beta"]);
}

#[test]
fn test_parse_tags_with_french_accents() {
    let response = r#"["réunion", "système", "déjà vu"]"#;
    let tags = parse_tags(response).unwrap();
    assert_eq!(tags, vec!["réunion", "système", "déjà vu"]);
}

#[test]
fn test_parse_tags_empty_array() {
    let response = "[]";
    let tags = parse_tags(response).unwrap();
    assert!(tags.is_empty());
}

#[test]
fn test_parse_tags_single_tag() {
    let response = r#"["only"]"#;
    let tags = parse_tags(response).unwrap();
    assert_eq!(tags, vec!["only"]);
}

#[test]
fn test_parse_tags_invalid_no_brackets() {
    let result = parse_tags("not json at all");
    assert!(matches!(result, Err(LlmError::TagParsing(_))));
}

#[test]
fn test_parse_tags_invalid_malformed_inside_brackets() {
    let result = parse_tags("[not, valid, json]");
    assert!(matches!(result, Err(LlmError::TagParsing(_))));
}

#[test]
fn test_parse_tags_invalid_empty_string() {
    let result = parse_tags("");
    assert!(matches!(result, Err(LlmError::TagParsing(_))));
}

#[test]
fn test_parse_tags_picks_outermost_brackets() {
    let response = r#"[["nested"], "outer"]"#;
    let tags = parse_tags(response);
    assert!(tags.is_err());
}
