use flowflow::application::hermes_chat::{
    self, HermesError, HermesTurn, LiveReply, RunEvent,
};
use flowflow::infrastructure::hermes::{
    drain_sse_data, normalize_base, parse_run_event, HermesMessage,
};
use flowflow::infrastructure::persistence::Database;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Real payloads captured from the Hermes API server on 2026-10-04.
const TOOL_STARTED: &str = r#"{"event": "tool.started", "run_id": "run_678a", "timestamp": 1791146491.09, "tool": "terminal", "preview": "date", "seq": 0}"#;
const TOOL_COMPLETED: &str = r#"{"event": "tool.completed", "run_id": "run_678a", "timestamp": 1791146491.85, "tool": "terminal", "duration": 0.811, "error": false, "preview": "{\"output\": \"Sun Oct  4 20:41:31 UTC 2026\", \"exit_code\": 0, \"error\": null}", "seq": 1}"#;
const REASONING: &str = r#"{"event": "reasoning.available", "run_id": "run_c4de", "timestamp": 1791146516.23, "text": "Je t'avais dit 20:41:31 UTC.", "seq": 0}"#;
const COMPLETED: &str = r#"{"event": "run.completed", "run_id": "run_c4de", "timestamp": 1791146516.28, "output": "Je t'avais dit 20:41:31 UTC.", "usage": {"input_tokens": 24003}, "seq": 1}"#;
const SESSION_MESSAGES: &str = r#"[
  {"id": 1, "session_id": "api_1", "role": "user", "content": "Lance la commande date dans le terminal.", "timestamp": 1791146483.3},
  {"id": 2, "session_id": "api_1", "role": "assistant", "content": "", "tool_calls": [{"id": "toolu_01", "call_id": "toolu_01", "type": "function", "function": {"name": "terminal", "arguments": "{\"command\": \"date\"}"}}], "timestamp": 1791146490.9},
  {"id": 3, "session_id": "api_1", "role": "tool", "tool_name": "terminal", "content": "{\"output\": \"Sun Oct  4 20:41:31 UTC 2026\", \"exit_code\": 0}", "timestamp": 1791146491.8},
  {"id": 4, "session_id": "api_1", "role": "assistant", "content": "Il est 20:41:31 UTC sur le serveur.", "timestamp": 1791146494.7}
]"#;

fn open_db(dir: &tempfile::TempDir) -> Database {
    Database::open_at(dir.path().join("flowflow_test.db")).expect("open_at")
}

#[test]
fn sse_frames_are_drained_whole_and_keepalives_skipped() {
    let mut buf = format!(
        ": open\n\nid: 0\ndata: {TOOL_STARTED}\n\n: keepalive\n\nid: 1\ndata: {{\"ev"
    );
    let frames = drain_sse_data(&mut buf);
    assert_eq!(frames, vec![TOOL_STARTED.to_string()]);
    assert_eq!(buf, "id: 1\ndata: {\"ev");
}

#[test]
fn run_events_map_to_what_the_chat_shows() {
    assert_eq!(
        parse_run_event(TOOL_STARTED),
        Some((
            0,
            RunEvent::ToolStarted {
                tool: "terminal".into(),
                preview: "date".into(),
            }
        ))
    );
    assert_eq!(
        parse_run_event(TOOL_COMPLETED),
        Some((
            1,
            RunEvent::ToolDone {
                tool: "terminal".into(),
                failed: false,
            }
        ))
    );
    assert_eq!(
        parse_run_event(COMPLETED),
        Some((
            1,
            RunEvent::Completed("Je t'avais dit 20:41:31 UTC.".into())
        ))
    );
    assert_eq!(parse_run_event(REASONING), None);
}

#[test]
fn session_history_folds_tool_calls_into_the_reply_steps() {
    let messages: Vec<HermesMessage> =
        serde_json::from_str(SESSION_MESSAGES).unwrap();
    let turns = hermes_chat::turns(&messages);
    assert_eq!(turns.len(), 2);
    assert_eq!(
        turns[0],
        HermesTurn::User {
            text: "Lance la commande date dans le terminal.".into(),
            attachments: vec![],
        }
    );
    let HermesTurn::Reply { text, steps } = &turns[1] else {
        panic!("expected a reply");
    };
    assert_eq!(text, "Il est 20:41:31 UTC sur le serveur.");
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].tool, "terminal");
    assert_eq!(steps[0].detail, "date");
    assert!(!steps[0].running);
}

#[test]
fn a_question_read_back_shows_attachment_names_never_their_body() {
    use flowflow::application::hermes_message::{compose, split, Attachment};
    let attachments = [
        Attachment::Photo {
            name: "Tableau.jpg".into(),
            jpeg: vec![1, 2, 3],
        },
        Attachment::File {
            name: "Devis \"v2\".pdf".into(),
            text: "Total 4 200 €".into(),
        },
        Attachment::Skill {
            name: "github-code-review".into(),
        },
    ];
    let (text, images) = compose("Résume ça", &attachments);
    assert!(
        text.contains("Total 4 200 €") && text.contains("github-code-review")
    );
    assert_eq!(images, vec!["data:image/jpeg;base64,AQID".to_string()]);
    assert_eq!(
        split(&text),
        (
            "Résume ça".to_string(),
            vec![
                "Tableau.jpg".into(),
                "Devis \"v2\".pdf".into(),
                "github-code-review".into()
            ]
        )
    );
    let (alone, _) = compose("", &attachments[1..2]);
    assert_eq!(split(&alone).0, "");
    assert_eq!(
        split("Juste une question"),
        ("Juste une question".into(), vec![])
    );
}

#[test]
fn skills_come_from_the_api_or_the_dashboard_page() {
    use flowflow::infrastructure::hermes::{
        dashboard_base, dashboard_token, parse_skills,
    };
    assert_eq!(
        dashboard_base("https://srv.tailnet.ts.net:8642").as_deref(),
        Some("https://srv.tailnet.ts.net")
    );
    let page = r#"<script>window.__HERMES_SESSION_TOKEN__="AbC-12_x";window.__HERMES_BASE_PATH__=""</script>"#;
    assert_eq!(dashboard_token(page).as_deref(), Some("AbC-12_x"));
    assert_eq!(dashboard_token("<html></html>"), None);
    // Dashboard shape: a bare array, a switched-off skill left out.
    let dashboard: serde_json::Value = serde_json::from_str(
        r#"[{"name": "codex", "description": "Delegate coding", "category": "autonomous-ai-agents", "enabled": true},
            {"name": "claude-code", "description": "", "category": "autonomous-ai-agents", "enabled": false}]"#,
    )
    .unwrap();
    let names: Vec<String> = parse_skills(&dashboard)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["codex"]);
    let api = serde_json::json!({"object": "list", "data": [{"name": "obsidian", "category": "note-taking"}]});
    assert_eq!(parse_skills(&api)[0].name, "obsidian");
}

#[test]
fn a_live_reply_streams_text_and_closes_its_steps() {
    let mut reply = LiveReply::default();
    for data in [TOOL_STARTED, TOOL_COMPLETED] {
        reply.apply(parse_run_event(data).unwrap().1);
    }
    reply.apply(RunEvent::Delta("Il est ".into()));
    reply.apply(RunEvent::Delta("20:41.".into()));
    assert_eq!(reply.text, "Il est 20:41.");
    assert_eq!(reply.steps.len(), 1);
    assert!(!reply.steps[0].running && !reply.steps[0].failed);
    assert!(!reply.done);

    reply.apply(RunEvent::Completed("Il est 20:41 UTC.".into()));
    assert_eq!(reply.text, "Il est 20:41 UTC.");
    assert!(reply.done);
}

#[test]
fn an_address_without_scheme_defaults_to_https() {
    assert_eq!(
        normalize_base(" srv.tailnet.ts.net:8642/ "),
        "https://srv.tailnet.ts.net:8642"
    );
    assert_eq!(
        normalize_base("http://127.0.0.1:8642"),
        "http://127.0.0.1:8642"
    );
}

#[test]
fn hermes_conversations_stay_on_the_device() {
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    let meta_rows = |db: &Database| -> i64 {
        db.conn()
            .query_row("SELECT COUNT(*) FROM sync_row_meta", [], |r| r.get(0))
            .unwrap()
    };
    let before = meta_rows(&db);

    db.create_hermes_conversation("flowflow_a", "Devis Lemaire")
        .unwrap();
    db.set_hermes_pending_run(
        "flowflow_a",
        Some(("run_1", "Prépare le devis")),
    )
    .unwrap();
    db.rename_hermes_conversation("flowflow_a", "Devis Cabinet Lemaire")
        .unwrap();

    assert_eq!(meta_rows(&db), before, "never enters the sync log");
    let listed = db.list_hermes_conversations().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "flowflow_a");
    assert_eq!(listed[0].title, "Devis Cabinet Lemaire");
    assert!(db.list_conversations().unwrap().is_empty());
    assert_eq!(
        db.hermes_pending_run("flowflow_a"),
        Some(("run_1".into(), "Prépare le devis".into()))
    );

    db.set_hermes_pending_run("flowflow_a", None).unwrap();
    assert_eq!(db.hermes_pending_run("flowflow_a"), None);
    db.delete_hermes_conversation("flowflow_a").unwrap();
    assert!(db.list_hermes_conversations().unwrap().is_empty());
}

/// A scripted Hermes: each accepted connection gets the next response whose
/// path matches; the request heads are kept for assertions.
struct Script {
    responses: Vec<(&'static str, String)>,
    seen: Arc<Mutex<Vec<String>>>,
}

fn sse(frames: &[&str]) -> String {
    let body: String = frames
        .iter()
        .map(|f| {
            let seq = serde_json::from_str::<serde_json::Value>(f).unwrap()
                ["seq"]
                .clone();
            format!("id: {seq}\ndata: {f}\n\n")
        })
        .collect();
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n: open\n\n{body}")
}

fn json(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

// Head and body may arrive in separate reads: read up to Content-Length.
async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await.unwrap();
        raw.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&raw).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if raw.len() >= end + 4 + length || n == 0 {
                return text;
            }
        } else if n == 0 {
            return text;
        }
    }
}

async fn serve(script: Script) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let mut responses = script.responses;
        while !responses.is_empty() {
            let (mut stream, _) = listener.accept().await.unwrap();
            let head = read_request(&mut stream).await;
            let line = head.lines().next().unwrap_or_default().to_string();
            script.seen.lock().unwrap().push(head);
            let at = responses
                .iter()
                .position(|(prefix, _)| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("unexpected request {line}"));
            let (_, response) = responses.remove(at);
            stream.write_all(response.as_bytes()).await.unwrap();
            let _ = stream.shutdown().await;
        }
    });
    base
}

fn configure(db: &Database, base: &str) {
    db.set_setting(hermes_chat::URL_SETTING, base).unwrap();
    db.set_setting(hermes_chat::KEY_SETTING, "k_0123456789abcdef")
        .unwrap();
}

#[tokio::test]
async fn a_refused_key_and_a_missing_server_say_so() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![(
            "GET /v1/models",
            json("401 Unauthorized", r#"{"error": "invalid key"}"#),
        )],
        seen: seen.clone(),
    })
    .await;
    assert_eq!(
        hermes_chat::check(&base, "wrong-key").await,
        Err(HermesError::KeyRefused)
    );
    assert!(seen.lock().unwrap()[0]
        .to_ascii_lowercase()
        .contains("authorization: bearer wrong-key"));

    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    assert_eq!(
        hermes_chat::check(&dead, "k").await,
        Err(HermesError::Unreachable)
    );

    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    assert_eq!(
        hermes_chat::ready(&db).await,
        Err(HermesError::NotConfigured)
    );
}

#[tokio::test]
async fn the_first_question_opens_a_flowflow_session_then_a_run() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![
            (
                "POST /api/sessions",
                json("201 Created", r#"{"object": "hermes.session"}"#),
            ),
            (
                "POST /v1/runs",
                json(
                    "202 Accepted",
                    r#"{"run_id": "run_1", "status": "started"}"#,
                ),
            ),
        ],
        seen: seen.clone(),
    })
    .await;
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    configure(&db, &base);

    let (session, run) =
        hermes_chat::send(&db, None, "Quelle heure est-il ?", &[], None, None)
            .await
            .unwrap();

    assert!(session.starts_with("flowflow_"));
    assert_eq!(run, "run_1");
    let listed = db.list_hermes_conversations().unwrap();
    assert_eq!(listed[0].id, session);
    assert_eq!(listed[0].title, "Quelle heure est-il ?");
    assert_eq!(
        hermes_chat::pending(&db, &session),
        Some(("run_1".into(), "Quelle heure est-il ?".into()))
    );
    let heads = seen.lock().unwrap();
    assert!(heads[1].contains(&format!("\"session_id\":\"{session}\"")));
}

#[tokio::test]
async fn a_dropped_stream_is_picked_up_where_it_stopped() {
    let started = r#"{"event": "tool.started", "run_id": "run_1", "tool": "terminal", "preview": "date", "seq": 0}"#;
    let first = r#"{"event": "message.delta", "run_id": "run_1", "delta": "Il est ", "seq": 1}"#;
    let done = r#"{"event": "tool.completed", "run_id": "run_1", "tool": "terminal", "error": false, "seq": 2}"#;
    let second = r#"{"event": "message.delta", "run_id": "run_1", "delta": "20:41.", "seq": 3}"#;
    let completed = r#"{"event": "run.completed", "run_id": "run_1", "output": "Il est 20:41.", "seq": 4}"#;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![
            // The phone sleeps: the stream ends before the run does.
            ("GET /v1/runs/run_1/events", sse(&[started, first])),
            (
                "GET /v1/runs/run_1 ",
                json("200 OK", r#"{"run_id": "run_1", "status": "running"}"#),
            ),
            ("GET /v1/runs/run_1/events", sse(&[done, second, completed])),
        ],
        seen: seen.clone(),
    })
    .await;
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    configure(&db, &base);
    db.create_hermes_conversation("flowflow_a", "Heure")
        .unwrap();
    db.set_hermes_pending_run("flowflow_a", Some(("run_1", "Quelle heure ?")))
        .unwrap();

    let mut reply = LiveReply::default();
    hermes_chat::follow(&db, "flowflow_a", "run_1", |e| reply.apply(e))
        .await
        .unwrap();

    assert_eq!(reply.text, "Il est 20:41.");
    assert_eq!(reply.steps.len(), 1);
    assert!(!reply.steps[0].running);
    assert!(reply.done && reply.error.is_none());
    assert_eq!(hermes_chat::pending(&db, "flowflow_a"), None);
    let heads = seen.lock().unwrap();
    assert!(heads[2].to_ascii_lowercase().contains("last-event-id: 1"));
}

#[tokio::test]
async fn a_run_the_server_forgot_ends_the_follow() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let gone = json("404 Not Found", r#"{"error": {"code": "run_not_found"}}"#);
    let base = serve(Script {
        responses: vec![
            ("GET /v1/runs/run_9/events", gone.clone()),
            ("GET /v1/runs/run_9 ", gone),
        ],
        seen,
    })
    .await;
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    configure(&db, &base);
    db.create_hermes_conversation("flowflow_b", "Ancien")
        .unwrap();
    db.set_hermes_pending_run("flowflow_b", Some(("run_9", "Question")))
        .unwrap();

    hermes_chat::follow(&db, "flowflow_b", "run_9", |_| {})
        .await
        .unwrap();
    assert_eq!(hermes_chat::pending(&db, "flowflow_b"), None);
}

#[test]
fn a_linking_qr_code_carries_the_address_and_the_key() {
    let link = "flowflow://hermes?url=https%3A%2F%2Fsrv.tailnet.ts.net%3A8642&key=k_0123456789abcdef";
    assert_eq!(
        hermes_chat::parse_link(link),
        Some((
            "https://srv.tailnet.ts.net:8642".to_string(),
            "k_0123456789abcdef".to_string()
        ))
    );
    assert_eq!(
        hermes_chat::parse_link("flowflow://hermes?url=https%3A%2F%2Fx"),
        None
    );
    assert_eq!(
        hermes_chat::parse_link("flowflow://share/abc?url=a&key=b"),
        None
    );
    assert_eq!(hermes_chat::parse_link("https://hermes?url=a&key=b"), None);
    assert_eq!(
        hermes_chat::parse_link(
            "flowflow://hermes?url=http%3A%2F%2Fevil.example%3A8642&key=k"
        ),
        None
    );
}

#[test]
fn model_ids_read_like_their_product_names() {
    assert_eq!(hermes_chat::model_label("claude-opus-5-5[1m]"), "Opus 5.5");
    assert_eq!(
        hermes_chat::model_label("claude-haiku-4-5-20251001"),
        "Haiku 4.5"
    );
    assert_eq!(hermes_chat::model_label("claude-sonnet-5[1m]"), "Sonnet 5");
    assert_eq!(hermes_chat::model_label("gpt-6.1-sol"), "GPT-6.1 Sol");
    assert_eq!(
        hermes_chat::model_label("gpt-6-astra-900k"),
        "GPT-6 Astra 900k"
    );
    assert_eq!(hermes_chat::model_label("llama-local"), "llama-local");
    assert_eq!(
        hermes_chat::provider_label(
            "claude-subscription-directsdk-experimental",
            "Claude Subscription DirectSDK (Experimental)"
        ),
        "Claude"
    );
    assert_eq!(
        hermes_chat::provider_label(
            "openai-codex",
            "ChatGPT or Codex Subscription"
        ),
        "ChatGPT"
    );
    assert_eq!(
        hermes_chat::provider_label("fireworks", "Fireworks AI (beta)"),
        "Fireworks AI"
    );
}

/// Shape captured from /api/model/options on 2026-10-05, trimmed.
#[test]
fn only_signed_in_providers_offer_models() {
    let raw = serde_json::json!({
        "provider": "claude-subscription-directsdk-experimental",
        "model": "claude-opus-5-5[1m]",
        "providers": [
            {"slug": "nous", "name": "Nous Portal", "authenticated": false, "models": []},
            {"slug": "openai-codex", "name": "ChatGPT or Codex Subscription", "authenticated": true, "models": ["gpt-6.1-sol", "gpt-6-astra"]},
            {"slug": "claude-subscription-directsdk-experimental", "name": "Claude Subscription DirectSDK (Experimental)", "authenticated": true, "models": ["claude-sonnet-5[1m]", "claude-opus-5-5[1m]"]}
        ]
    });
    let options = flowflow::infrastructure::hermes::parse_model_options(&raw);
    assert_eq!(options.model, "claude-opus-5-5[1m]");
    assert_eq!(options.providers.len(), 2);
    assert_eq!(
        options.providers[0].models,
        vec!["gpt-6.1-sol", "gpt-6-astra"]
    );
}

#[test]
fn a_pick_stays_with_its_conversation_and_starts_the_next_ones() {
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    assert_eq!(hermes_chat::chosen_model(&db, "flowflow_a"), None);
    hermes_chat::choose_model(&db, "flowflow_a", "openai-codex", "gpt-6.1-sol")
        .unwrap();
    hermes_chat::choose_effort(&db, "flowflow_a", "high").unwrap();
    // A new conversation starts on the latest pick, level included.
    assert_eq!(
        hermes_chat::chosen_model(&db, "flowflow_b"),
        Some(("openai-codex".into(), "gpt-6.1-sol".into()))
    );
    assert_eq!(
        hermes_chat::chosen_effort(&db, "flowflow_b").as_deref(),
        Some("high")
    );
    // A later pick elsewhere leaves this conversation's own pick alone.
    let claude = "claude-subscription-directsdk-experimental";
    hermes_chat::choose_model(&db, "flowflow_c", claude, "claude-opus-5-5[1m]")
        .unwrap();
    assert_eq!(
        hermes_chat::chosen_model(&db, "flowflow_a"),
        Some(("openai-codex".into(), "gpt-6.1-sol".into()))
    );
}

#[test]
fn back_to_back_calls_of_one_tool_fold_into_one_step() {
    let step = |tool: &str| hermes_chat::HermesStep {
        tool: tool.into(),
        detail: format!("{tool} detail"),
        running: false,
        failed: false,
    };
    let steps = vec![
        step("session_search"),
        step("read_file"),
        step("read_file"),
        step("read_file"),
        step("web_search"),
        step("read_file"),
    ];
    let grouped = hermes_chat::grouped(&steps);
    let shape: Vec<(&str, usize)> =
        grouped.iter().map(|(s, n)| (s.tool.as_str(), *n)).collect();
    assert_eq!(
        shape,
        vec![
            ("session_search", 1),
            ("read_file", 3),
            ("web_search", 1),
            ("read_file", 1)
        ]
    );
    assert_eq!(
        hermes_chat::tool_key("read_file"),
        Some("hermes-tool-read-file")
    );
    assert_eq!(hermes_chat::tool_key("yb_send_sticker"), None);
}

/// Model lists read from Hermes on 2026-10-05.
const CLAUDE_MODELS: [&str; 6] = [
    "claude-sonnet-5[1m]",
    "claude-haiku-4-5-20251001",
    "claude-opus-5-5[1m]",
    "claude-opus-5[1m]",
    "claude-opus-4-8[1m]",
    "claude-fable-5-1[1m]",
];
const CODEX_MODELS: [&str; 16] = [
    "gpt-6.1-sol",
    "gpt-6.1-sol-900k",
    "gpt-6-astra",
    "gpt-6-astra-900k",
    "gpt-6-sol",
    "gpt-6-sol-900k",
    "gpt-6-luna",
    "gpt-6-luna-900k",
    "gpt-5.6-sol",
    "gpt-5.6-sol-900k",
    "gpt-5.6-terra",
    "gpt-5.6-terra-900k",
    "gpt-5.6-luna",
    "gpt-5.6-luna-900k",
    "gpt-5.5",
    "gpt-5.3-codex-spark",
];

fn owned(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_menu_keeps_the_newest_model_of_each_family() {
    assert_eq!(
        hermes_chat::latest_models(&owned(&CLAUDE_MODELS), ""),
        owned(&[
            "claude-opus-5-5[1m]",
            "claude-fable-5-1[1m]",
            "claude-sonnet-5[1m]",
            "claude-haiku-4-5-20251001"
        ])
    );
    assert_eq!(
        hermes_chat::latest_models(&owned(&CODEX_MODELS), ""),
        owned(&[
            "gpt-6.1-sol",
            "gpt-6-astra",
            "gpt-6-luna",
            "gpt-5.6-terra",
            "gpt-5.5",
            "gpt-5.3-codex-spark"
        ])
    );
    // The model in use stays reachable even when a newer one hides it.
    assert!(hermes_chat::latest_models(
        &owned(&CLAUDE_MODELS),
        "claude-opus-4-8[1m]"
    )
    .contains(&"claude-opus-4-8[1m]".to_string()));
}

#[test]
fn the_provider_in_use_comes_first() {
    let options = flowflow::infrastructure::hermes::ModelOptions {
        provider: "claude-subscription-directsdk-experimental".into(),
        model: "claude-opus-5-5[1m]".into(),
        providers: vec![
            hermes_chat::ModelProvider {
                slug: "openai-codex".into(),
                name: "ChatGPT".into(),
                models: owned(&CODEX_MODELS),
            },
            hermes_chat::ModelProvider {
                slug: "claude-subscription-directsdk-experimental".into(),
                name: "Claude".into(),
                models: owned(&CLAUDE_MODELS),
            },
        ],
    };
    let menu = hermes_chat::menu_providers(
        &options,
        (
            "claude-subscription-directsdk-experimental",
            "claude-opus-5-5[1m]",
        ),
    );
    assert_eq!(menu[0].slug, "claude-subscription-directsdk-experimental");
    assert_eq!(menu[1].slug, "openai-codex");
    // OpenAI's current lineup is the GPT-6 generation only.
    assert_eq!(
        menu[1].models,
        owned(&["gpt-6.1-sol", "gpt-6-astra", "gpt-6-luna"])
    );
}

#[test]
fn each_model_offers_only_the_levels_hermes_accepts() {
    let claude = "claude-subscription-directsdk-experimental";
    assert_eq!(
        hermes_chat::efforts_for(claude, "claude-opus-5-5[1m]"),
        &["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(
        hermes_chat::efforts_for(claude, "claude-fable-5-1[1m]"),
        &["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(
        hermes_chat::efforts_for(claude, "claude-haiku-4-5-20251001"),
        &["low", "medium", "high", "xhigh"]
    );
    assert_eq!(
        hermes_chat::efforts_for("openai-codex", "gpt-6.1-sol"),
        &["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(
        hermes_chat::efforts_for("openai-codex", "gpt-6-luna"),
        &["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(
        hermes_chat::efforts_for("openai-codex", "gpt-5.5"),
        &["low", "medium", "high", "xhigh"]
    );
    assert!(hermes_chat::efforts_for("fireworks", "llama").is_empty());
}

#[test]
fn a_conversation_thinks_at_medium_unless_told_otherwise() {
    let claude = "claude-subscription-directsdk-experimental";
    assert_eq!(
        hermes_chat::effective_effort(None, claude, "claude-opus-5-5[1m]")
            .as_deref(),
        Some("medium")
    );
    assert_eq!(
        hermes_chat::effective_effort(
            Some("max"),
            claude,
            "claude-opus-5-5[1m]"
        )
        .as_deref(),
        Some("max")
    );
    // Haiku has no "max": the pick falls back to the default.
    assert_eq!(
        hermes_chat::effective_effort(
            Some("max"),
            claude,
            "claude-haiku-4-5-20251001"
        )
        .as_deref(),
        Some("medium")
    );
    assert_eq!(
        hermes_chat::effective_effort(None, "fireworks", "llama"),
        None
    );
}
