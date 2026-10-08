//! Talking to Hermes from the Chats tab. The conversation lives in a Hermes
//! session; FlowFlow keeps a pointer to it and follows the running turn,
//! which survives the app being suspended or killed.

use crate::application::hermes_message::{self, Asked, Attachment};
use crate::infrastructure::hermes::{HermesClient, HermesMessage, RunState};
use crate::infrastructure::persistence::Database;

pub use crate::infrastructure::hermes::{
    ApprovalRequest, HermesError, ModelOptions, ModelProvider, RunEvent, Skill,
    KEY_SETTING, URL_SETTING,
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
    /// The question typed and what went with it.
    User(Asked),
    Reply {
        text: String,
        steps: Vec<HermesStep>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApprovalStatus {
    Asked,
    Sending,
    /// Hermes stopped waiting: the command did not run.
    Expired,
}

/// A request for the user's go, on screen until it is answered.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingApproval {
    pub request: ApprovalRequest,
    pub status: ApprovalStatus,
    // The step running when Hermes asked: Hermes asks from inside the tool,
    // so that step ending unanswered means it gave up waiting.
    step: Option<usize>,
}

/// An answered request stays in the steps as a receipt, under this tool name
/// followed by the choice ("approval.once").
pub const RECEIPT_TOOL: &str = "approval.";

/// The reply being written while a run is live.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveReply {
    pub text: String,
    pub steps: Vec<HermesStep>,
    pub approvals: Vec<PendingApproval>,
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
                if let Some(i) =
                    self.steps.iter().rposition(|s| s.running && s.tool == tool)
                {
                    self.steps[i].running = false;
                    self.steps[i].failed = failed;
                    self.expire(|a| a.step == Some(i));
                }
            }
            RunEvent::Completed(output) => {
                if !output.trim().is_empty() {
                    self.text = output;
                }
                self.done = true;
                self.expire(|_| true);
            }
            RunEvent::Failed(error) => {
                self.error = Some(error);
                self.done = true;
                self.expire(|_| true);
            }
            RunEvent::ApprovalRequested(request) => {
                let known = self
                    .approvals
                    .iter()
                    .any(|a| a.request.request_id == request.request_id);
                if !known {
                    let step = self.steps.iter().rposition(|s| s.running);
                    self.approvals.push(PendingApproval {
                        request,
                        status: ApprovalStatus::Asked,
                        step,
                    });
                }
            }
            RunEvent::ApprovalResolved { request_id, choice } => {
                self.resolve(&request_id, &choice)
            }
        }
    }

    // An answer always wins over an expiry seen first: the tool can end
    // before the answer's own confirmation arrives.
    fn resolve(&mut self, request_id: &str, choice: &str) {
        let Some(i) = self
            .approvals
            .iter()
            .position(|a| a.request.request_id == request_id)
        else {
            return;
        };
        let answered = self.approvals.remove(i);
        self.steps.push(HermesStep {
            tool: format!("{RECEIPT_TOOL}{choice}"),
            detail: clip(&answered.request.command),
            running: false,
            failed: choice == "deny",
        });
    }

    // A request being answered is left alone: its answer decides.
    fn expire(&mut self, which: impl Fn(&PendingApproval) -> bool) {
        for a in self.approvals.iter_mut() {
            if a.status == ApprovalStatus::Asked && which(a) {
                a.status = ApprovalStatus::Expired;
            }
        }
    }

    /// Marks a request as being answered; false when it already is, or is
    /// no longer asked, so a second tap sends nothing.
    pub fn sending(&mut self, request_id: &str) -> bool {
        match self.approvals.iter_mut().find(|a| {
            a.request.request_id == request_id
                && a.status == ApprovalStatus::Asked
        }) {
            Some(a) => {
                a.status = ApprovalStatus::Sending;
                true
            }
            None => false,
        }
    }

    /// What Hermes made of an answer: taken, gone (it stopped waiting), or
    /// lost on the way, in which case it can be sent again.
    pub fn answered(
        &mut self,
        request_id: &str,
        choice: &str,
        result: &Result<(), HermesError>,
    ) {
        match result {
            Ok(()) => self.resolve(request_id, choice),
            Err(e) => {
                let status = if *e == HermesError::ApprovalGone {
                    ApprovalStatus::Expired
                } else {
                    ApprovalStatus::Asked
                };
                if let Some(a) = self
                    .approvals
                    .iter_mut()
                    .find(|a| a.request.request_id == request_id)
                {
                    a.status = status;
                }
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
                out.push(HermesTurn::User(hermes_message::split(&m.text())));
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
    // The key travels to this address: only an encrypted one is accepted.
    let https = url::Url::parse(&base).is_ok_and(|u| u.scheme() == "https");
    (https && !key.is_empty()).then_some((base, key))
}

/// An address and a key are saved: the Hermes choice is worth offering.
pub fn configured(db: &Database) -> bool {
    HermesClient::from_db(db).is_ok()
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
/// Stands for "no conversation yet": the latest pick lives under it, and
/// every conversation without its own pick starts from it.
pub const LAST_PICK: &str = "last";

pub fn chosen_model(
    db: &Database,
    session_id: &str,
) -> Option<(String, String)> {
    let raw = db
        .get_setting(&model_key(session_id))
        .or_else(|| db.get_setting(&model_key(LAST_PICK)))?;
    let (provider, model) = raw.split_once('\t')?;
    Some((provider.to_string(), model.to_string()))
}

/// Keeps the pick for the conversation and as the next one's start.
pub fn choose_model(
    db: &Database,
    session_id: &str,
    provider: &str,
    model: &str,
) -> Result<(), String> {
    let value = format!("{provider}\t{model}");
    db.set_setting(&model_key(LAST_PICK), &value)?;
    db.set_setting(&model_key(session_id), &value)
}

/// Hermes' reasoning levels, from the least to the most effort.
pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The reasoning levels a model accepts, as Hermes routes them; none for a
/// provider whose levels Hermes does not publish. Mirrors Hermes'
/// agent/reasoning_effort.py (Codex) and agent/anthropic_adapter.py (Claude),
/// which its API does not expose.
pub fn efforts_for(provider: &str, model: &str) -> &'static [&'static str] {
    // Thinking is never turned off from the app: every level thinks.
    const FULL: &[&str] = &["low", "medium", "high", "xhigh", "max"];
    const NO_MAX: &[&str] = &["low", "medium", "high", "xhigh"];
    const NO_XHIGH: &[&str] = &["low", "medium", "high", "max"];
    let m = model.to_lowercase();
    if provider.contains("codex") || provider.contains("openai") {
        if m.starts_with("gpt-6") || m.contains("gpt-5.6") {
            FULL
        } else {
            NO_MAX
        }
    } else if provider.contains("claude") || provider.contains("anthropic") {
        let legacy = ["claude-3", "-4-0", "-4-1", "-4-5", "-4-2025"]
            .iter()
            .any(|s| m.contains(s));
        if legacy {
            NO_MAX
        } else if m.contains("-4-6") {
            NO_XHIGH
        } else {
            FULL
        }
    } else {
        &[]
    }
}

/// The level a conversation starts on when nothing was picked.
pub const DEFAULT_EFFORT: &str = "medium";

/// The level a turn runs on: the one picked if the model accepts it, else
/// the default, else none for a model whose levels are unknown.
pub fn effective_effort(
    chosen: Option<&str>,
    provider: &str,
    model: &str,
) -> Option<String> {
    let levels = efforts_for(provider, model);
    chosen
        .filter(|e| levels.contains(e))
        .or_else(|| levels.contains(&DEFAULT_EFFORT).then_some(DEFAULT_EFFORT))
        .map(String::from)
}

fn effort_key(session_id: &str) -> String {
    format!("hermes_effort:{session_id}")
}

/// The reasoning level picked for a conversation; None follows Hermes.
pub fn chosen_effort(db: &Database, session_id: &str) -> Option<String> {
    db.get_setting(&effort_key(session_id))
        .filter(|e| EFFORTS.contains(&e.as_str()))
        .or_else(|| {
            db.get_setting(&effort_key(LAST_PICK))
                .filter(|e| EFFORTS.contains(&e.as_str()))
        })
}

/// Keeps the level for the conversation and as the next one's start.
pub fn choose_effort(
    db: &Database,
    session_id: &str,
    effort: &str,
) -> Result<(), String> {
    db.set_setting(&effort_key(LAST_PICK), effort)?;
    db.set_setting(&effort_key(session_id), effort)
}

// "claude-opus-5-5[1m]" -> ("claude-opus", [5, 5]); "gpt-6.1-sol" ->
// ("gpt-sol", [6, 1]). Release dates and context suffixes are not versions.
fn family_and_version(id: &str) -> (String, Vec<u32>) {
    let id = id.split('[').next().unwrap_or(id);
    let mut family = Vec::new();
    let mut version = Vec::new();
    for part in id.split('-') {
        if part.len() == 8 && part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let numbers: Vec<u32> =
            part.split('.').map_while(|n| n.parse().ok()).collect();
        if numbers.len() == part.split('.').count() && !numbers.is_empty() {
            version.extend(numbers);
        } else {
            family.push(part);
        }
    }
    (family.join("-"), version)
}

/// The newest model of each family, newest first; long-context twins
/// ("-900k") stay out, the model in use always stays in.
pub fn latest_models(models: &[String], keep: &str) -> Vec<String> {
    let mut best: Vec<(String, Vec<u32>, String)> = Vec::new();
    for m in models.iter().filter(|m| !m.ends_with("-900k")) {
        let (family, version) = family_and_version(m);
        match best.iter_mut().find(|(f, _, _)| *f == family) {
            Some(entry) if version > entry.1 => {
                *entry = (family, version, m.clone())
            }
            Some(_) => {}
            None => best.push((family, version, m.clone())),
        }
    }
    best.sort_by(|a, b| b.1.cmp(&a.1));
    let mut out: Vec<String> = best.into_iter().map(|(_, _, m)| m).collect();
    if models.iter().any(|m| m == keep) && !out.iter().any(|m| m == keep) {
        out.push(keep.to_string());
    }
    out
}

/// The menu's providers: the one in use first, each with its newest models.
pub fn menu_providers(
    options: &ModelOptions,
    in_use: (&str, &str),
) -> Vec<ModelProvider> {
    let mut providers: Vec<ModelProvider> = options
        .providers
        .iter()
        .map(|p| {
            let keep = if p.slug == in_use.0 { in_use.1 } else { "" };
            let mut models = latest_models(&p.models, keep);
            // OpenAI re-releases its tiers (Sol, Luna...) each generation:
            // only the newest generation is current. Claude tiers (Opus,
            // Sonnet, Haiku) move on their own, so each keeps its newest.
            if p.slug.contains("codex") || p.slug.contains("openai") {
                let major = |m: &String| {
                    family_and_version(m).1.first().copied().unwrap_or(0)
                };
                let newest = models.iter().map(major).max().unwrap_or(0);
                models.retain(|m| major(m) == newest || m == keep);
            }
            ModelProvider {
                slug: p.slug.clone(),
                name: p.name.clone(),
                models,
            }
        })
        .collect();
    providers.sort_by_key(|p| p.slug != in_use.0);
    providers
}

pub async fn model_options(db: &Database) -> Result<ModelOptions, HermesError> {
    HermesClient::from_db(db)?.model_options().await
}

/// The skills installed on Hermes.
pub async fn skills(db: &Database) -> Result<Vec<Skill>, HermesError> {
    HermesClient::from_db(db)?.skills().await
}

/// Scheduled tasks counted on Hermes, for the empty conversation; None when
/// Hermes cannot serve the list rather than a zero.
pub async fn jobs_count(db: &Database) -> Option<usize> {
    HermesClient::from_db(db)
        .ok()?
        .count("/api/jobs")
        .await
        .ok()
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

/// What Hermes is told about the app it answers through. Hermes tells every
/// API client to write plain text; this client renders Markdown.
pub fn client_instructions(phone: bool, lang: &str) -> String {
    let (device, screen) = if phone {
        (
            "iPhone",
            "The screen is a narrow phone: keep tables to 3 short columns, \
             prefer lists for wide data, keep code lines short.",
        )
    } else {
        ("Mac", "The screen is a desktop window.")
    };
    format!(
        "You are talking to the user through FlowFlow {} on {device}, a \
         voice notes app. Its chat renders GitHub-flavored Markdown \
         (headings, lists, bold, links, code blocks, and tables with the \
         |---| separator row), so the plain-text rule for API clients does \
         not apply here: use Markdown where it helps. {screen} The app is \
         set to the \"{lang}\" language.",
        env!("CARGO_PKG_VERSION")
    )
}

/// Sends a question and its attachments, on the model picked for the
/// conversation if any. The first one opens the Hermes session and the local
/// conversation. Returns the session and the run to follow.
pub async fn send(
    db: &Database,
    session_id: Option<String>,
    question: &str,
    attachments: &[Attachment],
    model: Option<(String, String)>,
    effort: Option<String>,
) -> Result<(String, String), HermesError> {
    let client = HermesClient::from_db(db)?;
    let (text, images) = hermes_message::compose(question, attachments);
    let session_id = match session_id {
        Some(id) => id,
        None => {
            let id = client.create_session().await?;
            let title =
                hermes_message::title(question, attachments, TITLE_CHARS);
            db.create_hermes_conversation(&id, &title)
                .map_err(HermesError::Server)?;
            if let Some((provider, name)) = &model {
                let _ = choose_model(db, &id, provider, name);
            }
            if let Some(e) = &effort {
                let _ = choose_effort(db, &id, e);
            }
            id
        }
    };
    let pick = model.or_else(|| chosen_model(db, &session_id));
    let effort = effort.or_else(|| chosen_effort(db, &session_id));
    let run_id = client
        .start_run(
            &session_id,
            &text,
            &images,
            pick.as_ref().map(|(p, m)| (p.as_str(), m.as_str())),
            effort.as_deref(),
            &client_instructions(
                cfg!(target_os = "ios"),
                &crate::application::i18n::ui_lang(db),
            ),
        )
        .await?;
    let _ =
        db.set_hermes_pending_run(&session_id, Some((run_id.as_str(), &text)));
    let _ = db.touch_hermes_conversation(&session_id);
    Ok((session_id, run_id))
}

/// Sends the user's choice for one approval request of a running turn.
pub async fn answer(
    db: &Database,
    run_id: &str,
    request_id: &str,
    choice: &str,
) -> Result<(), HermesError> {
    HermesClient::from_db(db)?
        .answer_approval(run_id, request_id, choice)
        .await
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
