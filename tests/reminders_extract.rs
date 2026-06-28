use chrono::{NaiveDate, NaiveTime};
use flowflow::application::reminders_extract::parse_reminder_intents;
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
