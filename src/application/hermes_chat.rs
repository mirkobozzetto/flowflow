//! Talking to Hermes from the Chats tab. The conversation lives in a Hermes
//! session; FlowFlow keeps a pointer to it and follows the running turn,
//! which survives the app being suspended or killed.

use crate::infrastructure::hermes::{HermesClient, HermesMessage, RunState};
use crate::infrastructure::persistence::Database;

pub use crate::infrastructure::hermes::{
    HermesError, ModelOptions, ModelProvider, RunEvent, KEY_SETTING,
    URL_SETTING,
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

/// Translation key naming a Hermes tool for a reader; None keeps the raw
/// name (a tool this list does not know yet).
pub fn tool_key(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "read_file" => "hermes-tool-read-file",
        "write_file" => "hermes-tool-write-file",
        "patch" => "hermes-tool-patch",
        "search_files" => "hermes-tool-search-files",
        "terminal" => "hermes-tool-terminal",
        "execute_code" => "hermes-tool-execute-code",
        "web_search" => "hermes-tool-web-search",
        "web_extract" => "hermes-tool-web-extract",
        "x_search" => "hermes-tool-x-search",
        "session_search" => "hermes-tool-session-search",
        "memory" => "hermes-tool-memory",
        "skill_view" | "skills_list" => "hermes-tool-skill",
        "todo_list" => "hermes-tool-todo",
        "vision_analyze" => "hermes-tool-vision",
        "image_generate" => "hermes-tool-image",
        _ => return None,
    })
}

/// Back-to-back calls of one tool fold into one step and a count; the
/// first call's detail stands for the group.
pub fn grouped(steps: &[HermesStep]) -> Vec<(HermesStep, usize)> {
    let mut out: Vec<(HermesStep, usize)> = Vec::new();
    for step in steps {
        match out.last_mut() {
            Some((last, n)) if last.tool == step.tool => {
                *n += 1;
                last.running |= step.running;
                last.failed |= step.failed;
            }
            _ => out.push((step.clone(), 1)),
        }
    }
    out
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

fn model_key(session_id: &str) -> String {
    format!("hermes_model:{session_id}")
}

/// The (provider, model) picked for a conversation; None follows Hermes'
/// own default. Device-local, like the conversation itself.
pub fn chosen_model(
    db: &Database,
    session_id: &str,
) -> Option<(String, String)> {
    let raw = db.get_setting(&model_key(session_id))?;
    let (provider, model) = raw.split_once('\t')?;
    Some((provider.to_string(), model.to_string()))
}

pub fn choose_model(
    db: &Database,
    session_id: &str,
    provider: &str,
    model: &str,
) -> Result<(), String> {
    db.set_setting(&model_key(session_id), &format!("{provider}\t{model}"))
}

pub async fn model_options(db: &Database) -> Result<ModelOptions, HermesError> {
    HermesClient::from_db(db)?.model_options().await
}

/// What the empty conversation shows: skills and scheduled tasks counted on
/// Hermes; a list it cannot serve is left out rather than shown as zero.
pub async fn counts(db: &Database) -> (Option<usize>, Option<usize>) {
    let Ok(client) = HermesClient::from_db(db) else {
        return (None, None);
    };
    let (skills, jobs) =
        futures::join!(client.count("/v1/skills"), client.count("/api/jobs"));
    (skills.ok(), jobs.ok())
}

/// "claude-opus-5-5[1m]" reads "Opus 5.5", "gpt-6.1-sol" reads "GPT-6.1 Sol".
pub fn model_label(id: &str) -> String {
    let id = id.split('[').next().unwrap_or(id);
    let words = |rest: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for part in rest.split('-').filter(|p| !p.is_empty()) {
            // A trailing release date (20251001) says nothing to a reader.
            if part.len() == 8 && part.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            match out.last_mut() {
                Some(last)
                    if part.chars().all(|c| c.is_ascii_digit())
                        && last
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_ascii_digit()) =>
                {
                    last.push('.');
                    last.push_str(part);
                }
                Some(last) if part.chars().all(|c| c.is_ascii_digit()) => {
                    last.push(' ');
                    last.push_str(part);
                }
                _ => {
                    let mut c = part.chars();
                    let first = c.next().map(|f| f.to_uppercase().to_string());
                    out.push(first.unwrap_or_default() + c.as_str());
                }
            }
        }
        out
    };
    if let Some(rest) = id.strip_prefix("claude-") {
        return words(rest).join(" ");
    }
    if let Some(rest) = id.strip_prefix("gpt-") {
        let mut parts = rest.splitn(2, '-');
        let version = parts.next().unwrap_or_default();
        let tail = words(parts.next().unwrap_or_default()).join(" ");
        return format!("GPT-{version} {tail}").trim().to_string();
    }
    id.to_string()
}

/// A provider as the menu names it: the family, without the plumbing.
pub fn provider_label(slug: &str, name: &str) -> String {
    if slug.contains("claude") || slug.contains("anthropic") {
        "Claude".into()
    } else if slug.contains("codex") || slug.contains("openai") {
        "ChatGPT".into()
    } else {
        name.split(" (").next().unwrap_or(name).to_string()
    }
}

/// Sends a question, on the model picked for the conversation if any. The
/// first one opens the Hermes session and the local conversation. Returns
/// the session and the run to follow.
pub async fn send(
    db: &Database,
    session_id: Option<String>,
    text: &str,
    model: Option<(String, String)>,
) -> Result<(String, String), HermesError> {
    let client = HermesClient::from_db(db)?;
    let session_id = match session_id {
        Some(id) => id,
        None => {
            let id = client.create_session().await?;
            let title: String = text.chars().take(TITLE_CHARS).collect();
            db.create_hermes_conversation(&id, &title)
                .map_err(HermesError::Server)?;
            if let Some((provider, name)) = &model {
                let _ = choose_model(db, &id, provider, name);
            }
            id
        }
    };
    let pick = model.or_else(|| chosen_model(db, &session_id));
    let run_id = client
        .start_run(
            &session_id,
            text,
            pick.as_ref().map(|(p, m)| (p.as_str(), m.as_str())),
        )
        .await?;
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
