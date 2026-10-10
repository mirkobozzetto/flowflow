// Notifications: the device token reaches the FlowFlow server, turning them off removes it,
// and the server's refusals read as plain reasons.

mod support;

use flowflow::application::push::{self, PushError};
use flowflow::infrastructure::persistence::Database;
use std::sync::{Arc, Mutex};
use support::{json, serve, Script};
use tempfile::tempdir;

const TOKEN: &str = "aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11";

fn empty(status: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
}

// A device that already holds a live session: no auth handshake in the script.
async fn device(
    dir: &tempfile::TempDir,
    responses: Vec<(&'static str, String)>,
) -> (Database, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses,
        seen: seen.clone(),
    })
    .await;
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    db.set_setting("backend_base_url", &base).unwrap();
    db.set_setting("backend_session_token", "session").unwrap();
    db.set_setting("backend_session_expires_at", "2999-01-01T00:00:00+00:00")
        .unwrap();
    (db, seen)
}

#[tokio::test]
async fn registering_sends_the_token_and_turns_notifications_on() {
    let dir = tempdir().unwrap();
    let (db, seen) =
        device(&dir, vec![("POST /v1/push/devices", empty("204 No Content"))])
            .await;

    push::register(&db, TOKEN).await.unwrap();

    let request = seen.lock().unwrap()[0].clone();
    assert!(request.contains(&format!(r#""token":"{TOKEN}""#)), "{request}");
    // Tests build in debug, like `make all`: a development profile, sandbox APNs.
    assert!(request.contains(r#""environment":"sandbox""#), "{request}");
    assert!(push::is_enabled(&db));
}

#[tokio::test]
async fn turning_notifications_off_removes_the_token() {
    let dir = tempdir().unwrap();
    let (db, seen) = device(
        &dir,
        vec![
            ("POST /v1/push/devices", empty("204 No Content")),
            ("DELETE /v1/push/devices", empty("204 No Content")),
        ],
    )
    .await;
    push::register(&db, TOKEN).await.unwrap();

    push::disable(&db).await.unwrap();

    assert!(seen.lock().unwrap()[1].starts_with("DELETE /v1/push/devices"));
    assert!(!push::is_enabled(&db));
}

#[tokio::test]
async fn notifications_stay_on_while_the_server_still_holds_the_token() {
    let dir = tempdir().unwrap();
    let (db, _) = device(
        &dir,
        vec![
            ("POST /v1/push/devices", empty("204 No Content")),
            (
                "DELETE /v1/push/devices",
                json("500 Internal Server Error", r#"{"error":"internal error"}"#),
            ),
        ],
    )
    .await;
    push::register(&db, TOKEN).await.unwrap();

    assert!(push::disable(&db).await.is_err());
    assert!(push::is_enabled(&db));
}

#[tokio::test]
async fn a_test_alert_waits_long_enough_to_close_the_app() {
    let dir = tempdir().unwrap();
    let (db, seen) = device(
        &dir,
        vec![("POST /v1/push/test", json("202 Accepted", r#"{"devices":1}"#))],
    )
    .await;

    push::send_test(&db, "Test").await.unwrap();

    let request = seen.lock().unwrap()[0].clone();
    assert!(request.contains(r#""body":"Test""#), "{request}");
    assert!(
        request.contains(&format!(r#""delay_secs":{}"#, push::TEST_DELAY_SECS)),
        "{request}"
    );
}

#[tokio::test]
async fn the_servers_refusals_read_as_reasons() {
    for (response, expected) in [
        (json("403 Forbidden", r#"{"error":"forbidden"}"#), PushError::NotPremium),
        (
            json("409 Conflict", r#"{"error":"no_push_device"}"#),
            PushError::NoDevice,
        ),
        (
            json("409 Conflict", r#"{"error":"push_unavailable"}"#),
            PushError::Unavailable,
        ),
    ] {
        let dir = tempdir().unwrap();
        let (db, _) = device(&dir, vec![("POST /v1/push/test", response)]).await;

        assert_eq!(push::send_test(&db, "Test").await, Err(expected));
    }
}

#[tokio::test]
async fn without_a_backend_nothing_is_sent() {
    if std::env::var("FLOWFLOW_BACKEND_URL").is_ok() {
        return;
    }
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();

    assert_eq!(push::register(&db, TOKEN).await, Err(PushError::NoBackend));
    assert!(!push::is_enabled(&db));
}
