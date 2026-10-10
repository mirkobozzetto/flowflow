use chrono::{DateTime, FixedOffset};
use flowflow::application::hermes_chat;
use flowflow::application::hermes_jobs::{
    self, clock_shift, next_run_text, schedule_text, status_text,
};
use flowflow::infrastructure::hermes::{parse_jobs, HermesJob, JobSchedule};
use flowflow::infrastructure::persistence::Database;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

mod support;
use support::{json, serve, Script};

/// `GET /api/jobs` as Mirko's Hermes answered it on 2026-10-10 (prompts cut,
/// Telegram ids replaced). Hermes runs on UTC there.
const JOBS: &str = include_str!("fixtures/hermes_jobs.json");

fn jobs() -> Vec<HermesJob> {
    parse_jobs(&serde_json::from_str(JOBS).unwrap())
}

fn job(name_start: &str) -> HermesJob {
    jobs()
        .into_iter()
        .find(|j| j.name.starts_with(name_start))
        .unwrap()
}

fn cron(expr: &str) -> JobSchedule {
    JobSchedule {
        kind: "cron".into(),
        expr: Some(expr.into()),
        minutes: None,
        display: expr.into(),
    }
}

fn every(minutes: u32) -> JobSchedule {
    JobSchedule {
        kind: "interval".into(),
        expr: None,
        minutes: Some(minutes),
        display: format!("every {minutes}m"),
    }
}

// Brussels in summer, where Mirko reads his tasks.
fn brussels() -> FixedOffset {
    FixedOffset::east_opt(2 * 3600).unwrap()
}

#[test]
fn jobs_read_as_hermes_sends_them() {
    assert_eq!(jobs().len(), 4);
    let morning = job("Point quotidien");
    assert_eq!(morning.id, "7455526871f4");
    assert_eq!(morning.schedule.expr.as_deref(), Some("0 5,6 * * 1-5"));
    assert_eq!(
        morning.next_run_at.as_deref(),
        Some("2026-10-12T05:00:00+00:00")
    );
    assert_eq!(morning.last_status.as_deref(), Some("ok"));
    assert!(!morning.paused());
    let watch = job("Suivi correctifs");
    assert_eq!(watch.schedule.kind, "interval");
    assert_eq!(watch.schedule.minutes, Some(360));
}

#[test]
fn a_job_paused_on_hermes_reads_paused() {
    // What Hermes' pause_job writes on the job.
    let mut v: serde_json::Value = serde_json::from_str(JOBS).unwrap();
    v["jobs"][0]["enabled"] = false.into();
    v["jobs"][0]["state"] = "paused".into();
    v["jobs"][0]["paused_at"] = "2026-10-10T08:00:00+00:00".into();
    assert!(parse_jobs(&v)[0].paused());
}

#[test]
fn hermes_clock_is_read_from_its_next_runs() {
    assert_eq!(clock_shift(&jobs(), &brussels()), 120);
    assert_eq!(clock_shift(&jobs(), &FixedOffset::east_opt(0).unwrap()), 0);
    assert_eq!(clock_shift(&[], &brussels()), 0);
}

#[test]
fn a_cron_schedule_reads_in_local_time() {
    let fr = |expr: &str| schedule_text("fr", &cron(expr), 120);
    let en = |expr: &str| schedule_text("en", &cron(expr), 120);
    assert_eq!(fr("0 5,6 * * 1-5"), "Du lundi au vendredi à 7 h et 8 h");
    assert_eq!(en("0 5,6 * * 1-5"), "Monday to Friday at 7 AM and 8 AM");
    assert_eq!(fr("30 5 * * 1,3,5"), "Lundi, mercredi et vendredi à 7 h 30");
    assert_eq!(
        en("30 5 * * 1,3,5"),
        "Monday, Wednesday and Friday at 7:30 AM"
    );
    assert_eq!(
        fr("0 13,14,15,16 * * 1-5"),
        "Du lundi au vendredi à 15 h, 16 h, 17 h et 18 h"
    );
    assert_eq!(fr("0 7 * * *"), "Tous les jours à 9 h");
    assert_eq!(en("0 10 * * *"), "Every day at 12 PM");
    assert_eq!(fr("15 6 * * 0"), "Le dimanche à 8 h 15");
    assert_eq!(en("15 6 * * 7"), "Every Sunday at 8:15 AM");
}

#[test]
fn an_interval_reads_in_hours_or_minutes() {
    assert_eq!(schedule_text("fr", &every(360), 120), "Toutes les 6 h");
    assert_eq!(schedule_text("en", &every(360), 120), "Every 6 hours");
    assert_eq!(schedule_text("fr", &every(45), 120), "Toutes les 45 min");
    assert_eq!(schedule_text("en", &every(60), 120), "Every hour");
}

#[test]
fn a_schedule_it_cannot_say_keeps_hermes_words() {
    // The last one crosses midnight once moved to Brussels.
    for expr in ["*/15 9-17 * * *", "0 9 1 * *", "0 23 * * 1-5"] {
        assert_eq!(schedule_text("fr", &cron(expr), 120), expr);
    }
}

#[test]
fn the_next_run_reads_as_a_day_and_an_hour() {
    let now = DateTime::parse_from_rfc3339("2026-10-10T12:00:00+02:00").unwrap();
    let next = |iso: &str| {
        let mut j = job("Point quotidien");
        j.next_run_at = Some(iso.into());
        j
    };
    let fr = |iso: &str| next_run_text("fr", &next(iso), &now).unwrap();
    assert_eq!(fr("2026-10-10T10:35:08+00:00"), "Aujourd'hui à 12 h 35");
    assert_eq!(fr("2026-10-11T05:00:00+00:00"), "Demain à 7 h");
    assert_eq!(fr("2026-10-12T05:00:00+00:00"), "Lundi à 7 h");
    assert_eq!(fr("2026-10-30T05:00:00+00:00"), "30 oct. à 7 h");
    assert_eq!(
        next_run_text("en", &next("2026-10-12T05:00:00+00:00"), &now).unwrap(),
        "Monday at 7 AM"
    );
    let mut paused = next("2026-10-12T05:00:00+00:00");
    paused.enabled = false;
    assert_eq!(next_run_text("fr", &paused, &now), None);
}

#[test]
fn the_last_run_says_how_it_went() {
    let mut j = job("Point quotidien");
    assert_eq!(status_text("fr", &j), "Réussie");
    j.last_status = Some("error".into());
    assert_eq!(status_text("fr", &j), "Échec");
    assert_eq!(status_text("en", &j), "Failed");
    j.last_status = None;
    assert_eq!(status_text("fr", &j), "Jamais lancée");
}

#[test]
fn a_winter_phone_is_one_hour_ahead_of_utc_hermes() {
    let winter = FixedOffset::east_opt(3600).unwrap();
    let mut j = job("Point quotidien");
    j.next_run_at = Some("2026-12-14T06:00:00+00:00".into());
    assert_eq!(clock_shift(&[j], &winter), 60);
}

fn linked_db(dir: &tempfile::TempDir, base: &str) -> Database {
    let db = Database::open_at(dir.path().join("flowflow_test.db")).unwrap();
    db.set_setting(hermes_chat::URL_SETTING, base).unwrap();
    db.set_setting(hermes_chat::KEY_SETTING, "k_0123456789abcdef")
        .unwrap();
    db
}

#[tokio::test]
async fn the_list_asks_hermes_for_paused_jobs_too() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![(
            "GET /api/jobs?include_disabled=true ",
            json("200 OK", JOBS),
        )],
        seen: seen.clone(),
    })
    .await;
    let dir = tempdir().unwrap();
    let db = linked_db(&dir, &base);
    assert_eq!(hermes_jobs::list(&db).await.unwrap().len(), 4);
    assert!(seen.lock().unwrap()[0]
        .to_ascii_lowercase()
        .contains("authorization: bearer k_0123456789abcdef"));
}
