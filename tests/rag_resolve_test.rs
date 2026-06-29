use flowflow::application::rag::{resolve_allowed_ids, resolve_web_on};

#[test]
fn web_fires_only_when_chat_asked_and_key_present() {
    assert!(
        !resolve_web_on(false, "exa-key"),
        "off chat never hits the web"
    );
    assert!(!resolve_web_on(true, ""), "no key -> no web");
    assert!(!resolve_web_on(true, "   "), "blank key -> no web");
    assert!(resolve_web_on(true, "exa-key"), "on + key -> web");
}

#[test]
fn mentions_override_scope_else_scope_stands() {
    let scope = Some(vec!["a".to_string(), "b".to_string()]);

    assert_eq!(
        resolve_allowed_ids(None, vec![]),
        None,
        "global chat = all notes"
    );
    assert_eq!(
        resolve_allowed_ids(scope.clone(), vec![]),
        scope,
        "no mention -> scope note set stands"
    );
    assert_eq!(
        resolve_allowed_ids(None, vec!["c".to_string()]),
        Some(vec!["c".to_string()]),
        "mention in a global chat narrows to the note"
    );
    assert_eq!(
        resolve_allowed_ids(scope, vec!["c".to_string()]),
        Some(vec!["c".to_string()]),
        "mention overrides the chat scope"
    );
}
