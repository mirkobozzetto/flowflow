use flowflow::application::hermes_delivery::{self, NtfyLink};
use flowflow::infrastructure::hermes::{parse_jobs, HermesClient, HermesJob};
use std::sync::{Arc, Mutex};

mod support;
use support::{json, serve, Script};

/// `GET /api/jobs` as Mirko's Hermes answered it on 2026-10-10.
const JOBS: &str = include_str!("fixtures/hermes_jobs.json");
const KEY: &str = "k_0123456789abcdef";

fn job(name_start: &str) -> HermesJob {
    parse_jobs(&serde_json::from_str(JOBS).unwrap())
        .into_iter()
        .find(|j| j.name.starts_with(name_start))
        .unwrap()
}

#[test]
fn turning_flowflow_on_adds_it_next_to_the_other_destinations() {
    assert_eq!(hermes_delivery::with_flowflow("local", true), "ntfy");
    assert_eq!(hermes_delivery::with_flowflow("", true), "ntfy");
    assert_eq!(
        hermes_delivery::with_flowflow("origin", true),
        "origin,ntfy"
    );
    assert_eq!(
        hermes_delivery::with_flowflow("origin,ntfy", true),
        "origin,ntfy"
    );
}

#[test]
fn turning_flowflow_off_keeps_the_other_destinations() {
    assert_eq!(
        hermes_delivery::with_flowflow("telegram,ntfy", false),
        "telegram"
    );
    assert_eq!(hermes_delivery::with_flowflow("ntfy", false), "local");
    assert_eq!(hermes_delivery::with_flowflow("origin", false), "origin");
}

#[test]
fn a_job_reads_its_destinations_as_hermes_sends_them() {
    let morning = job("Point quotidien");
    assert_eq!(morning.deliver, "origin");
    assert!(!hermes_delivery::delivers_to_flowflow(&morning));
    // Created from Telegram: "origin" is Telegram.
    assert!(hermes_delivery::telegram_too(&morning));
    assert!(!hermes_delivery::telegram_too(&job("Trois posts")));
    assert!(hermes_delivery::telegram_too(&job("Suivi correctifs")));

    let listed = parse_jobs(&serde_json::json!({"jobs": [{
        "id": "a", "name": "A", "deliver": ["telegram", "ntfy"],
        "schedule": {"kind": "cron", "expr": "0 5 * * *", "display": "0 5 * * *"}
    }]}));
    assert_eq!(listed[0].deliver, "telegram,ntfy");
    assert!(hermes_delivery::delivers_to_flowflow(&listed[0]));
}

#[test]
fn the_ntfy_link_comes_from_the_key_hermes_and_the_app_share() {
    let link = NtfyLink::from_key(KEY);
    assert_eq!(link.topic, "a35facf211c3e1edd9471306d6d3bdb7");
    assert_eq!(
        link.token,
        "c354a3eca1ecc1da6b6d1fa27f6a8a5cd64741e49ca8c9d20670e10645d9e87e"
    );
    assert_eq!(
        link.token_hash(),
        "9ff71b8e718bb2d17ff087aa69344ccd760d0ff4cbd31639cef2c94781877ade"
    );
}

#[test]
fn hermes_gets_the_settings_of_its_ntfy_module() {
    let link = NtfyLink::from_key(KEY);
    assert_eq!(
        hermes_delivery::hermes_env(&link, "https://api.flowflow.be/"),
        vec![
            (
                "NTFY_SERVER_URL",
                "https://api.flowflow.be/ntfy".to_string()
            ),
            ("NTFY_TOPIC", link.topic.clone()),
            ("NTFY_TOKEN", link.token.clone()),
            ("NTFY_MARKDOWN", "true".to_string()),
            ("NTFY_HOME_CHANNEL", link.topic.clone()),
        ]
    );
}

#[tokio::test]
async fn the_destinations_change_on_hermes() {
    let updated = r#"{"job": {"id": "7455526871f4", "name": "Point", "deliver": "origin,ntfy", "schedule": {"kind": "cron", "expr": "0 5 * * *", "display": "0 5 * * *"}}}"#;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![(
            "PATCH /api/jobs/7455526871f4 ",
            json("200 OK", updated),
        )],
        seen: seen.clone(),
    })
    .await;

    let job = HermesClient::new(&base, KEY)
        .unwrap()
        .update_deliver("7455526871f4", "origin,ntfy")
        .await
        .unwrap();

    assert_eq!(job.deliver, "origin,ntfy");
    assert!(seen.lock().unwrap()[0].contains(r#"{"deliver":"origin,ntfy"}"#));
}

#[tokio::test]
async fn a_new_job_can_deliver_to_flowflow_from_the_start() {
    let created = r#"{"job": {"id": "n1", "name": "Revue", "deliver": "ntfy", "schedule": {"kind": "cron", "expr": "0 5 * * 5", "display": "0 5 * * 5"}}}"#;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![("POST /api/jobs ", json("200 OK", created))],
        seen: seen.clone(),
    })
    .await;

    HermesClient::new(&base, KEY)
        .unwrap()
        .create_job("Revue", "Fais la revue.", "0 5 * * 5", Some("ntfy"))
        .await
        .unwrap();

    assert!(seen.lock().unwrap()[0].contains(r#""deliver":"ntfy""#));
}
