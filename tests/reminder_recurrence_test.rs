use chrono::Weekday;
use flowflow::domain::{RecurrenceFreq, RecurrenceSpec};

#[test]
fn parses_daily() {
    let s = RecurrenceSpec::parse("DAILY").unwrap();
    assert_eq!(s.freq, RecurrenceFreq::Daily);
    assert!(s.weekdays.is_empty());
    assert!(s.month_days.is_empty());
}

#[test]
fn parses_weekly_single_day() {
    let s = RecurrenceSpec::parse("WEEKLY;BYDAY=MO").unwrap();
    assert_eq!(s.freq, RecurrenceFreq::Weekly);
    assert_eq!(s.weekdays, vec![Weekday::Mon]);
}

#[test]
fn parses_weekly_weekdays() {
    let s = RecurrenceSpec::parse("WEEKLY;BYDAY=MO,TU,WE,TH,FR").unwrap();
    assert_eq!(
        s.weekdays,
        vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri
        ]
    );
}

#[test]
fn parses_monthly_monthday() {
    let s = RecurrenceSpec::parse("MONTHLY;BYMONTHDAY=1").unwrap();
    assert_eq!(s.freq, RecurrenceFreq::Monthly);
    assert_eq!(s.month_days, vec![1]);
}

#[test]
fn lowercase_and_whitespace_tolerated() {
    let s = RecurrenceSpec::parse(" weekly ; byday = mo , fr ").unwrap();
    assert_eq!(s.freq, RecurrenceFreq::Weekly);
    assert_eq!(s.weekdays, vec![Weekday::Mon, Weekday::Fri]);
}

#[test]
fn rejects_empty_unknown_freq_and_null() {
    assert!(RecurrenceSpec::parse("").is_none());
    assert!(RecurrenceSpec::parse("YEARLY").is_none());
    assert!(RecurrenceSpec::parse("null").is_none());
}

#[test]
fn ignores_unknown_params_and_bad_days() {
    let s = RecurrenceSpec::parse("WEEKLY;INTERVAL=2;BYDAY=MO,XX").unwrap();
    assert_eq!(s.weekdays, vec![Weekday::Mon]);
}

#[test]
fn dedups_repeated_days() {
    let s = RecurrenceSpec::parse("WEEKLY;BYDAY=MO,MO").unwrap();
    assert_eq!(s.weekdays, vec![Weekday::Mon]);
}

#[test]
fn drops_out_of_range_monthday() {
    let s = RecurrenceSpec::parse("MONTHLY;BYMONTHDAY=0,32,15").unwrap();
    assert_eq!(s.month_days, vec![15]);
}
