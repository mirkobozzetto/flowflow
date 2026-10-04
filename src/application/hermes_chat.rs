//! Talking to Hermes from the Chats tab. The conversation lives in a Hermes
//! session; FlowFlow keeps a pointer to it and follows the running turn,
//! which survives the app being suspended or killed.

use crate::infrastructure::hermes::{HermesClient, HermesMessage, RunState};
use crate::infrastructure::persistence::Database;

pub use crate::infrastructure::hermes::{
    HermesError, RunEvent, KEY_SETTING, URL_SETTING,
};

/// The link the kit's QR code carries: flowflow://hermes?url=…&key=…
pub const LINK_PREFIX: &str = "flowflow://hermes";

const TITLE_CHARS: usize = 50;
const STEP_DETAIL_CHARS: usize = 80;
// A dropped stream is followed again; after this many failed checks in a row
// (about a minute) the turn is left pending for the next opening.
const MAX_MISSES: u32 = 30;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq)]
pub struct HermesStep {
    pub tool: String,
    pub detail: String,
    pub running: bool,
    pub failed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HermesTurn {
    User(String),
    Reply {
        text: String,
        steps: Vec<HermesStep>,
    },
}

/// The reply being written while a run is live.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveReply {
    pub text: String,
    pub steps: Vec<HermesStep>,
    pub error: Option<String>,
    pub done: bool,
}

impl LiveReply {
    pub fn apply(&mut self, event: RunEvent) {
        match event {
            RunEvent::Delta(d) => self.text.push_str(&d),
            RunEvent::ToolStarted { tool, preview } => {
                self.steps.push(HermesStep {
                    tool,
                    detail: clip(&preview),
                    running: true,
                    failed: false,
                })
            }
            RunEvent::ToolDone { tool, failed } => {
                if let Some(step) = self
                    .steps
                    .iter_mut()
                    .rev()
                    .find(|s| s.running && s.tool == tool)
                {
                    step.running = false;
                    step.failed = failed;
                }
            }
            RunEvent::Completed(output) => {
                if !output.trim().is_empty() {
                    self.text = output;
                }
                self.done = true;
            }
            RunEvent::Failed(error) => {
                self.error = Some(error);
                self.done = true;
            }
        }
    }
}

fn clip(s: &str) -> String {
    let line = s.lines().next().unwrap_or_default().trim();
    if line.chars().count() > STEP_DETAIL_CHARS {
        let cut: String = line.chars().take(STEP_DETAIL_CHARS).collect();
        format!("{cut}…")
    } else {
        line.to_string()
    }
}

// Tool arguments arrive as a JSON string or object; the first string value
// ("date" for {"command": "date"}) says what the step did.
fn step_detail(arguments: &serde_json::Value) -> String {
    let parsed = match arguments {
        serde_json::Value::String(s) => serde_json::from_str(s)
            .unwrap_or(serde_json::Value::String(s.clone())),
        other => other.clone(),
    };
    let first = match &parsed {
        serde_json::Value::Object(map) => map
            .values()
            .find_map(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    };
    clip(&first)
}

/// A Hermes session's messages as chat turns: one reply per question, the
/// tools used on the way folded into its steps.
pub fn turns(messages: &[HermesMessage]) -> Vec<HermesTurn> {
    let mut out = Vec::new();
    let mut steps: Vec<HermesStep> = Vec::new();
    for m in messages {
        match m.role.as_str() {
            "user" => {
                if !steps.is_empty() {
                    out.push(HermesTurn::Reply {
                        text: String::new(),
                        steps: std::mem::take(&mut steps),
                    });
                }
                out.push(HermesTurn::User(m.text()));
            }
            "assistant" => {
                let calls = m.tool_calls.as_deref().unwrap_or_default();
                if calls.is_empty() {
                    let text = m.text();
                    if !text.trim().is_empty() {
                        out.push(HermesTurn::Reply {
                            text,
                            steps: std::mem::take(&mut steps),
                        });
                    }
                } else {
                    steps.extend(calls.iter().map(|c| HermesStep {
                        tool: c.function.name.clone(),
                        detail: step_detail(&c.function.arguments),
                        running: false,
                        failed: false,
                    }));
                }
            }
            _ => {}
        }
    }
    if !steps.is_empty() {
        out.push(HermesTurn::Reply {
            text: String::new(),
            steps,
        });
    }
    out
}

/// Address and key carried by a linking QR code; None for any other link.
pub fn parse_link(uri: &str) -> Option<(String, String)> {
    let parsed = url::Url::parse(uri.trim()).ok()?;
    if parsed.scheme() != "flowflow" || parsed.host_str() != Some("hermes") {
        return None;
    }
    let (mut base, mut key) = (String::new(), String::new());
    for (name, value) in parsed.query_pairs() {
        match name.as_ref() {
            "url" => base = value.trim().to_string(),
            "key" => key = value.trim().to_string(),
            _ => {}
        }
    }
    (!base.is_empty() && !key.is_empty()).then_some((base, key))
}

/// Saves the address and key a linking QR code brought.
pub fn link(db: &Database, url: &str, key: &str) -> Result<(), String> {
    db.set_setting(URL_SETTING, url)?;
    db.set_setting(KEY_SETTING, key)
}

/// Reachable and the key accepted, for the Settings test.
pub async fn check(url: &str, key: &str) -> Result<(), HermesError> {
    HermesClient::new(url, key)?.check().await
}

/// Hermes is set up and answers, for opening a conversation.
pub async fn ready(db: &Database) -> Result<(), HermesError> {
    HermesClient::from_db(db)?.check().await
}

pub async fn history(
    db: &Database,
    session_id: &str,
) -> Result<Vec<HermesTurn>, HermesError> {
    let client = HermesClient::from_db(db)?;
    Ok(turns(&client.messages(session_id).await?))
}

/// The question of a turn still running, so a reopened conversation can
/// show it before Hermes has stored it.
pub fn pending(db: &Database, session_id: &str) -> Option<(String, String)> {
    db.hermes_pending_run(session_id)
}

/// Sends a question. The first one opens the Hermes session and the local
/// conversation. Returns the session and the run to follow.
pub async fn send(
    db: &Database,
    session_id: Option<String>,
    text: &str,
) -> Result<(String, String), HermesError> {
    let client = HermesClient::from_db(db)?;
    let session_id = match session_id {
        Some(id) => id,
        None => {
            let id = client.create_session().await?;
            let title: String = text.chars().take(TITLE_CHARS).collect();
            db.create_hermes_conversation(&id, &title)
                .map_err(HermesError::Server)?;
            id
        }
    };
    let run_id = client.start_run(&session_id, text).await?;
    let _ =
        db.set_hermes_pending_run(&session_id, Some((run_id.as_str(), text)));
    let _ = db.touch_hermes_conversation(&session_id);
    Ok((session_id, run_id))
}

/// Follows a run to its end, picking the stream up again after a drop (the
/// app suspended mid-answer). The pending run is cleared once it is over.
pub async fn follow(
    db: &Database,
    session_id: &str,
    run_id: &str,
    mut on_event: impl FnMut(RunEvent),
) -> Result<(), HermesError> {
    let client = HermesClient::from_db(db)?;
    let mut last = -1i64;
    let mut misses = 0u32;
    loop {
        let mut ended = false;
        let _ = client
            .follow_run(run_id, last, |seq, event| {
                last = last.max(seq);
                ended |= matches!(
                    event,
                    RunEvent::Completed(_) | RunEvent::Failed(_)
                );
                on_event(event);
            })
            .await;
        if ended {
            break;
        }
        match client.run_state(run_id).await {
            Ok(RunState::Running) => misses = 0,
            Ok(RunState::Completed(output)) => {
                on_event(RunEvent::Completed(output));
                break;
            }
            Ok(RunState::Failed(error)) => {
                on_event(RunEvent::Failed(error));
                break;
            }
            // The server no longer knows the run (restarted): the session
            // history holds whatever it finished.
            Err(HermesError::NotFound) => break,
            Err(e) => {
                misses += 1;
                if misses >= MAX_MISSES {
                    return Err(e);
                }
            }
        }
        futures_timer::Delay::new(RETRY_DELAY).await;
    }
    let _ = db.set_hermes_pending_run(session_id, None);
    let _ = db.touch_hermes_conversation(session_id);
    Ok(())
}
