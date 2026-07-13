// Activation (08-activation): resolve a chat message to one installed, active agent, or to
// nothing. Deterministic first - exact alias/id (bare or "lance <alias>"), then the keyword
// prefilter over the manifest's own triggers; only a single-agent keyword candidate reaches
// the LLM confidence judge. No match ever guesses: the caller falls through to the generic
// assistant. Trigger words are authored on the platform and travel in the signed manifest -
// nothing here hardcodes an agent's vocabulary.

use crate::application::constants::AGENT_TRIGGER_JUDGE_PROMPT;
use crate::application::intent::TRIGGER_VERBS;
use crate::domain::agent_manifest::{parse_manifest, AgentManifest};
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;

/// A resolved activation: which agent, which of its chains.
#[derive(Debug, Clone, PartialEq)]
pub struct Activation {
    pub agent_id: String,
    pub chain: String,
}

/// The deterministic verdict, before any LLM is consulted.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// Exact alias/id match: fire without a judge.
    Direct(Activation),
    /// Exactly one agent's keyword prefilter matched: fire only if the judge agrees.
    Candidate(Activation),
    /// No match, or an ambiguous match (two agents): fall through, never guess.
    None,
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

// The chain an activation runs when the trigger does not name one: the manifest's only
// chain, else the first trigger's `then` that exists. None -> the agent cannot activate.
fn default_chain(m: &AgentManifest) -> Option<String> {
    if m.orchestration.chains.len() == 1 {
        return m.orchestration.chains.keys().next().cloned();
    }
    m.orchestration
        .triggers
        .iter()
        .find(|t| m.orchestration.chains.contains_key(&t.then))
        .map(|t| t.then.clone())
}

fn activation_for(
    m: &AgentManifest,
    chain: Option<String>,
) -> Option<Activation> {
    Some(Activation {
        agent_id: m.id.clone(),
        chain: chain.or_else(|| default_chain(m))?,
    })
}

// The bare alias/id, "<launch verb> <alias|id>", or that same launch form followed by a free
// goal ("lance synchro-clients quelle est ma liste ?"): the user naturally appends the question
// to the activation, and an exact-only match would silently fall through to the generic
// assistant. The boundary check keeps `agent-x` from hijacking `agent-x-v2`.
fn alias_match(msg: &str, m: &AgentManifest) -> bool {
    let targets: Vec<String> = [m.alias.as_str(), m.id.as_str()]
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect();
    targets.iter().any(|t| {
        msg == t
            || TRIGGER_VERBS.iter().any(|v| {
                let head = format!("{v} {t}");
                msg == head
                    || msg.strip_prefix(&head).is_some_and(|rest| {
                        rest.starts_with(|c: char| {
                            c.is_whitespace()
                                || (c.is_ascii_punctuation()
                                    && c != '-'
                                    && c != '_')
                        })
                    })
            })
    })
}

// Keyword prefilter: any authored trigger word/phrase contained in the message
// (case-insensitive). Only in-run keyword triggers count in v1; the chain is the
// trigger's `then` when it exists.
fn keyword_match(msg: &str, m: &AgentManifest) -> Option<Activation> {
    for trig in &m.orchestration.triggers {
        if trig.kind != "keyword" || trig.scope != "in_run" {
            continue;
        }
        if trig
            .words
            .iter()
            .any(|w| !w.trim().is_empty() && msg.contains(&w.to_lowercase()))
        {
            let chain = m
                .orchestration
                .chains
                .contains_key(&trig.then)
                .then(|| trig.then.clone());
            return activation_for(m, chain);
        }
    }
    None
}

/// Deterministic resolution over the installed+active manifests. Pure, so it is testable
/// offline; `resolve` wraps it with the DB read and the LLM confidence stage.
pub fn resolve_deterministic(
    message: &str,
    manifests: &[AgentManifest],
) -> Resolution {
    let msg = normalize(message);
    if msg.is_empty() {
        return Resolution::None;
    }
    // Aliases are display metadata, not unique: two published agents can carry the same
    // one. Exactly one match fires; a collision falls through (never guess).
    let alias_hits: Vec<&AgentManifest> =
        manifests.iter().filter(|m| alias_match(&msg, m)).collect();
    match alias_hits.as_slice() {
        [one] => {
            return match activation_for(one, None) {
                Some(a) => Resolution::Direct(a),
                None => Resolution::None,
            };
        }
        [] => {}
        _ => return Resolution::None,
    }
    let candidates: Vec<Activation> = manifests
        .iter()
        .filter_map(|m| keyword_match(&msg, m))
        .collect();
    match candidates.as_slice() {
        [one] => Resolution::Candidate(one.clone()),
        _ => Resolution::None,
    }
}

fn installed_manifests(db: &Database) -> Vec<AgentManifest> {
    db.list_installed_agents()
        .into_iter()
        .filter(|a| a.active)
        .filter_map(|a| parse_manifest(&a.manifest_json).ok())
        .collect()
}

// Stage two of a keyword trigger: an LLM confirms the phrasing really asks for this agent.
// Unavailable or unparseable judge -> false: the safe outcome is the generic assistant.
async fn confidence_holds(
    db: &Database,
    m: &AgentManifest,
    message: &str,
) -> bool {
    let Ok(llm) = LlmClient::from_db(db) else {
        return false;
    };
    let prompt = AGENT_TRIGGER_JUDGE_PROMPT
        .replace("{name}", &m.name)
        .replace("{description}", &m.description);
    match llm.chat(&prompt, message).await {
        Ok(verdict) => {
            let v = verdict.trim().to_lowercase();
            v.starts_with("yes") || v.starts_with("oui")
        }
        Err(_) => false,
    }
}

/// Resolve a chat message to an activation, or None to fall through to the generic
/// assistant. Alias/id matches fire directly; a single keyword candidate fires only
/// when the confidence judge agrees.
pub async fn resolve(db: &Database, message: &str) -> Option<Activation> {
    let manifests = installed_manifests(db);
    if manifests.is_empty() {
        return None;
    }
    match resolve_deterministic(message, &manifests) {
        Resolution::Direct(a) => Some(a),
        Resolution::Candidate(a) => {
            let m = manifests.iter().find(|m| m.id == a.agent_id)?;
            confidence_holds(db, m, message).await.then_some(a)
        }
        Resolution::None => None,
    }
}

/// Run the resolved activation with the user's message as the goal. Returns the answer plus
/// the run's ground-truth trace (states + tool verdicts) for the debug accordion; `events`
/// carries tool status + approval proposals to the UI.
pub async fn run(
    db: &Database,
    activation: &Activation,
    goal: &str,
    events: tokio::sync::mpsc::UnboundedSender<
        crate::application::tools::ToolEvent,
    >,
) -> Result<(String, crate::application::rag::RagTrace), String> {
    let outcome = crate::application::connector_module::run_agent_chain(
        db,
        &activation.agent_id,
        &activation.chain,
        goal,
        events,
    )
    .await?;
    let trace = chain_trace(&outcome);
    Ok((outcome.final_text, trace))
}

// The chain's tool log reshaped into the RAG trace the debug toggle already renders: one
// step per chain state, one per tool verdict line ("held ...", "blocked: ...").
fn chain_trace(
    outcome: &crate::application::chain::ChainOutcome,
) -> crate::application::rag::RagTrace {
    use crate::application::rag::{RagTrace, StepOutcome, TraceStep};
    let mut trace = RagTrace::default();
    let mut push = |label: String, outcome: StepOutcome| {
        trace.push(TraceStep {
            step: label,
            model: None,
            tokens_in: None,
            tokens_out: None,
            approx: false,
            latency_ms: 0,
            retries: 0,
            outcome,
        });
    };
    for step in &outcome.trace {
        push(format!("state `{}`", step.state), StepOutcome::Ok);
        for line in &step.tools {
            let verdict = if line.starts_with("blocked") {
                StepOutcome::Skipped
            } else {
                StepOutcome::Ok
            };
            push(line.clone(), verdict);
        }
    }
    trace
}

/// The installed+active agents the + palette offers, with the exact command a tap sends.
/// The command targets the ID, never the alias: ids are unique on device, aliases are
/// not, and a palette tap must be deterministic.
pub fn palette_entries(db: &Database) -> Vec<(String, String)> {
    installed_manifests(db)
        .into_iter()
        .map(|m| (m.name, format!("lance {}", m.id)))
        .collect()
}
