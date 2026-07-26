use chrono::{Datelike, Local, NaiveDate};
use flowflow::application::rag::{
    apply_date_filter, detect_temporal_regex, DateRange,
};
use flowflow::infrastructure::vectordb::{SearchResult, SourceType};

/// Words that merely CONTAIN a month name are not a date intent.
///
/// "mai" hides inside some of the most ordinary French words there are, and the
/// month test used a bare substring check. A query holding "mais" was read as
/// "the month of May", every note outside May was dropped, and the chat answered
/// "no relevant note" over an empty set. Nothing on screen said a date filter had
/// run.
#[test]
fn a_word_that_merely_contains_a_month_is_not_a_date() {
    let false_positives = [
        "j'ai noté un truc mais je ne sais plus où",
        "demain je dois envoyer le dossier",
        "je n'ai jamais écrit ça",
        "qu'est-ce que je fais maintenant",
        "retrouve le mail de la banque",
        "mes notes sur la maison",
        "quel est mon domaine d'expertise",
        "je n'aimais pas cette idée",
    ];
    for q in false_positives {
        assert_eq!(
            detect_temporal_regex(q),
            None,
            "no date intent in {q:?}, yet one was detected"
        );
    }
}

#[test]
fn a_real_month_is_still_detected() {
    let range = detect_temporal_regex("mes notes de mai").expect("mai");
    assert_eq!(range.from.month(), 5);
    assert_eq!(range.from.day(), 1);
    assert_eq!(range.to.month(), 5);
    assert_eq!(range.to.day(), 31);

    let range =
        detect_temporal_regex("ce que j'ai fait en mars").expect("mars");
    assert_eq!(range.from.month(), 3);
    assert_eq!(range.to.day(), 31);

    // Punctuation is a word boundary too.
    assert!(detect_temporal_regex("et en juin, quoi ?").is_some());
}

#[test]
fn every_month_name_is_reachable() {
    let months = [
        ("janvier", 1),
        ("février", 2),
        ("fevrier", 2),
        ("mars", 3),
        ("avril", 4),
        ("mai", 5),
        ("juin", 6),
        ("juillet", 7),
        ("août", 8),
        ("aout", 8),
        ("septembre", 9),
        ("octobre", 10),
        ("novembre", 11),
        ("décembre", 12),
        ("decembre", 12),
    ];
    for (name, month) in months {
        let q = format!("mes notes de {name}");
        let range = detect_temporal_regex(&q)
            .unwrap_or_else(|| panic!("{name} not detected"));
        assert_eq!(range.from.month(), month, "wrong month for {name}");
    }
}

/// The relative-period guards run before the month table and must keep working:
/// "cette semaine" is a date intent even though "semaine" alone is not.
#[test]
fn relative_period_guards_still_fire() {
    let today = Local::now().date_naive();

    let range = detect_temporal_regex("mes notes de cette semaine")
        .expect("cette semaine");
    assert_eq!(range.to, today);
    assert!(range.from <= today);

    let range = detect_temporal_regex("ce que j'ai noté hier").expect("hier");
    assert_eq!(range.from, today - chrono::Duration::days(1));
    assert_eq!(range.to, range.from);

    assert!(detect_temporal_regex("le mois dernier").is_some());
    assert!(detect_temporal_regex("ce mois-ci").is_some());
    assert!(detect_temporal_regex("aujourd'hui").is_some());
}

#[test]
fn a_question_with_no_date_yields_none() {
    assert_eq!(detect_temporal_regex("de quoi parlait la réunion"), None);
    assert_eq!(detect_temporal_regex(""), None);
}

fn result_created(id: &str, created_at: &str) -> SearchResult {
    SearchResult {
        note_id: id.to_string(),
        title: id.to_string(),
        chunk_text: String::new(),
        distance: 0.0,
        relevance: 1.0,
        created_at: created_at.to_string(),
        source_type: SourceType::Local,
        url: None,
    }
}

#[test]
fn the_date_filter_keeps_only_what_falls_inside_the_range() {
    let range = DateRange {
        from: NaiveDate::from_ymd_opt(2026, 5, 1).expect("date"),
        to: NaiveDate::from_ymd_opt(2026, 5, 31).expect("date"),
    };
    let kept = apply_date_filter(
        vec![
            result_created("before", "2026-04-30T10:00:00Z"),
            result_created("first-day", "2026-05-01T00:00:00Z"),
            result_created("inside", "2026-05-15T12:00:00Z"),
            result_created("last-day", "2026-05-31T23:59:00Z"),
            result_created("after", "2026-06-01T09:00:00Z"),
        ],
        &range,
    );
    assert_eq!(
        kept.iter().map(|r| r.note_id.as_str()).collect::<Vec<_>>(),
        ["first-day", "inside", "last-day"]
    );
}
