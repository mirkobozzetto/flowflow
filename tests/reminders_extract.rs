use chrono::{NaiveDate, NaiveTime};
use flowflow::application::reminders_extract::{
    group_same_slot, parse_reminder_intents,
};
use flowflow::domain::DEFAULT_REMINDER_HOUR;

#[test]
fn parses_single_intent_with_time() {
    let raw = r#"{"intents":[{"action":"appeler Paul","date":"2026-06-02","time":"15:00","recurrence":null,"location":null}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].action, "appeler Paul");
    assert_eq!(v[0].resolved_date(), NaiveDate::from_ymd_opt(2026, 6, 2));
    assert_eq!(
        v[0].resolved_time(),
        NaiveTime::from_hms_opt(15, 0, 0).unwrap()
    );
    assert!(v[0].has_explicit_time());
    assert!(v[0].has_date());
}

#[test]
fn defaults_missing_time_to_nine() {
    let raw = r#"{"intents":[{"action":"faire les courses","date":"2026-06-06","time":null}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 1);
    assert!(!v[0].has_explicit_time());
    assert_eq!(
        v[0].resolved_time(),
        NaiveTime::from_hms_opt(DEFAULT_REMINDER_HOUR, 0, 0).unwrap()
    );
}

#[test]
fn parses_multiple_intents_with_recurrence() {
    let raw = r#"{"intents":[
        {"action":"call the dentist","date":"2026-06-02","time":null},
        {"action":"pay rent","date":"2026-07-01","time":null,"recurrence":"MONTHLY;BYMONTHDAY=1"}
    ]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].recurrence.as_deref(), Some("MONTHLY;BYMONTHDAY=1"));
}

#[test]
fn tolerates_markdown_fence() {
    let raw = "```json\n{\"intents\":[{\"action\":\"x\",\"date\":\"2026-06-02\"}]}\n```";
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 1);
}

#[test]
fn empty_intents_ok() {
    let v = parse_reminder_intents(r#"{"intents":[]}"#).unwrap();
    assert!(v.is_empty());
}

#[test]
fn no_date_flagged_by_has_date() {
    let raw = r#"{"intents":[{"action":"vague","date":null}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 1);
    assert!(!v[0].has_date());
}

#[test]
fn invalid_json_errors() {
    assert!(parse_reminder_intents("not json at all").is_err());
}

#[test]
fn groups_sub_items_into_one_intent_with_notes_body() {
    let raw = r#"{"intents":[{"action":"rendez-vous","items":["acheter X","appeler Y","préparer Z"],"date":"2026-06-02","time":"14:00"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].items, vec!["acheter X", "appeler Y", "préparer Z"]);
    assert_eq!(
        v[0].notes_body().as_deref(),
        Some("- acheter X\n- appeler Y\n- préparer Z")
    );
}

#[test]
fn single_action_has_no_items_and_no_notes() {
    let raw = r#"{"intents":[{"action":"appeler Paul","date":"2026-06-02","time":"15:00"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert!(v[0].items.is_empty());
    assert!(v[0].notes_body().is_none());
}

#[test]
fn notes_body_drops_blank_items() {
    let raw = r#"{"intents":[{"action":"x","items":["a","  ",""],"date":"2026-06-02"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v[0].notes_body().as_deref(), Some("- a"));
}

#[test]
fn merges_same_slot_fanout_into_one_event() {
    let raw = r#"{"intents":[
        {"action":"acheter X","date":"2026-06-02","time":"14:00"},
        {"action":"appeler Y","date":"2026-06-02","time":"14:00"},
        {"action":"préparer Z","date":"2026-06-02","time":"14:00"}
    ]}"#;
    let grouped = group_same_slot(parse_reminder_intents(raw).unwrap());
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].action, "acheter X");
    assert_eq!(grouped[0].items, vec!["appeler Y", "préparer Z"]);
    assert_eq!(
        grouped[0].notes_body().as_deref(),
        Some("- appeler Y\n- préparer Z")
    );
}

#[test]
fn keeps_distinct_slots_separate() {
    let raw = r#"{"intents":[
        {"action":"call dentist","date":"2026-06-02","time":null},
        {"action":"pay rent","date":"2026-07-01","time":null}
    ]}"#;
    let grouped = group_same_slot(parse_reminder_intents(raw).unwrap());
    assert_eq!(grouped.len(), 2);
}

#[test]
fn recurrence_without_until_is_not_effective() {
    let raw = r#"{"intents":[{"action":"standup","date":"2026-06-08","time":"09:00","recurrence":"WEEKLY;BYDAY=MO"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert!(v[0].recurrence.is_some());
    assert!(v[0].effective_recurrence().is_none());
}

#[test]
fn recurrence_with_until_is_effective() {
    let raw = r#"{"intents":[{"action":"point","date":"2026-06-03","time":"09:00","recurrence":"WEEKLY;BYDAY=WE","until":"2026-06-30"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert_eq!(v[0].effective_recurrence(), Some("WEEKLY;BYDAY=WE"));
    assert_eq!(v[0].resolved_until(), NaiveDate::from_ymd_opt(2026, 6, 30));
}

#[test]
fn until_without_recurrence_is_harmless() {
    let raw = r#"{"intents":[{"action":"x","date":"2026-06-03","until":"2026-06-30"}]}"#;
    let v = parse_reminder_intents(raw).unwrap();
    assert!(v[0].effective_recurrence().is_none());
    assert!(v[0].resolved_until().is_some());
}

#[test]
fn folds_a_model_grouped_intent_and_a_stray_at_the_same_slot() {
    let raw = r#"{"intents":[
        {"action":"rendez-vous","items":["acheter X","appeler Y"],"date":"2026-06-02","time":"14:00"},
        {"action":"préparer Z","date":"2026-06-02","time":"14:00"}
    ]}"#;
    let grouped = group_same_slot(parse_reminder_intents(raw).unwrap());
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].action, "rendez-vous");
    assert_eq!(
        grouped[0].items,
        vec!["acheter X", "appeler Y", "préparer Z"]
    );
}
