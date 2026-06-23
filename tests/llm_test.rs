use flowflow::application::error::LlmError;
use flowflow::infrastructure::llm::{parse_tags, Provider};
use std::str::FromStr;

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

#[test]
fn test_llm_error_display_not_configured() {
    let err = LlmError::NotConfigured("missing key".into());
    assert_eq!(format!("{err}"), "missing key");
}

#[test]
fn test_llm_error_display_embedding() {
    let err = LlmError::Embedding("network fail".into());
    assert_eq!(format!("{err}"), "Embedding error: network fail");
}

#[test]
fn test_llm_error_display_completion() {
    let err = LlmError::Completion("rate limit".into());
    assert_eq!(format!("{err}"), "Completion error: rate limit");
}

#[test]
fn test_llm_error_display_tag_parsing() {
    let err = LlmError::TagParsing("bad json".into());
    assert_eq!(format!("{err}"), "Tag parsing error: bad json");
}

#[test]
fn test_llm_error_to_string_conversion() {
    let err = LlmError::Embedding("oops".into());
    let s: String = err.into();
    assert_eq!(s, "Embedding error: oops");
}

#[test]
fn test_llm_error_is_std_error() {
    fn assert_is_error<E: std::error::Error>(_: &E) {}
    let err = LlmError::Completion("x".into());
    assert_is_error(&err);
}

#[test]
fn test_llm_error_debug_format() {
    let err = LlmError::NotConfigured("k".into());
    let dbg = format!("{err:?}");
    assert!(dbg.contains("NotConfigured"));
}

#[test]
fn test_provider_default_is_openai() {
    assert_eq!(Provider::default(), Provider::OpenAi);
}

#[test]
fn test_provider_display_openai() {
    assert_eq!(format!("{}", Provider::OpenAi), "openai");
}

#[test]
fn test_provider_display_anthropic() {
    assert_eq!(format!("{}", Provider::Anthropic), "anthropic");
}

#[test]
fn test_provider_as_str_openai() {
    assert_eq!(Provider::OpenAi.as_str(), "openai");
}

#[test]
fn test_provider_as_str_anthropic() {
    assert_eq!(Provider::Anthropic.as_str(), "anthropic");
}

#[test]
fn test_provider_from_str_openai_canonical() {
    assert_eq!(Provider::from_str("openai").unwrap(), Provider::OpenAi);
}

#[test]
fn test_provider_from_str_openai_variants() {
    assert_eq!(Provider::from_str("OpenAI").unwrap(), Provider::OpenAi);
    assert_eq!(Provider::from_str("open_ai").unwrap(), Provider::OpenAi);
    assert_eq!(Provider::from_str("open-ai").unwrap(), Provider::OpenAi);
    assert_eq!(Provider::from_str("  openai  ").unwrap(), Provider::OpenAi);
}

#[test]
fn test_provider_from_str_anthropic_canonical() {
    assert_eq!(
        Provider::from_str("anthropic").unwrap(),
        Provider::Anthropic
    );
}

#[test]
fn test_provider_from_str_anthropic_variants() {
    assert_eq!(
        Provider::from_str("Anthropic").unwrap(),
        Provider::Anthropic
    );
    assert_eq!(Provider::from_str("claude").unwrap(), Provider::Anthropic);
    assert_eq!(Provider::from_str("CLAUDE").unwrap(), Provider::Anthropic);
}

#[test]
fn test_provider_from_str_invalid() {
    assert!(Provider::from_str("gemini").is_err());
    assert!(Provider::from_str("").is_err());
    assert!(Provider::from_str("unknown").is_err());
}

#[test]
fn test_provider_roundtrip_via_as_str() {
    for p in [Provider::OpenAi, Provider::Anthropic] {
        let parsed = Provider::from_str(p.as_str()).unwrap();
        assert_eq!(parsed, p);
    }
}

#[test]
fn test_provider_clone_copy_eq() {
    let p = Provider::Anthropic;
    let q = p;
    let r = p;
    assert_eq!(p, q);
    assert_eq!(p, r);
    assert_ne!(Provider::OpenAi, Provider::Anthropic);
}

#[test]
fn test_provider_debug_format() {
    let dbg = format!("{:?}", Provider::OpenAi);
    assert!(dbg.contains("OpenAi"));
    let dbg = format!("{:?}", Provider::Anthropic);
    assert!(dbg.contains("Anthropic"));
}
