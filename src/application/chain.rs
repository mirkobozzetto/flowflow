// The chain runtime: drives a validated FSM (`domain::orchestration`) over a connector's MCP tools. Rust
// owns the path - for each state it mounts only that state's tools, runs one gated agent turn, threads a
// transcript forward, enforces the read_before_write guard at the FSM seam, and composes a terminal answer.
// It reports a per-state trace, including states the backend serves no tool for, so gaps surface honestly.

use crate::application::agent_builder::BuiltAgent;
use crate::application::error::LlmError;
use crate::application::tools::ContractHook;
use crate::domain::orchestration::{Chain, Guard};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::{resolve_chat_model, LlmClient, NotesTools};
use crate::infrastructure::mcp::McpPool;
use crate::infrastructure::persistence::Database;
use tokio::sync::mpsc;

const ANSWER_PREAMBLE: &str =
    "You are the final step of a tool chain. ANSWER THE USER'S GOAL first, using the data the \
     tools actually returned in the chain context. A failed or skipped step is only worth \
     mentioning if it prevents answering the goal - never lead with it. Do not call any tool. \
     Any id or URL you mention MUST be copied character-for-character from the context; if you \
     are not certain of a URL, name the resource without linking it. Reply in the user's \
     language.";

pub struct ChainStep {
    pub state: String,
    pub outcome: String,
    // Ground-truth tool log for this state: what was called/blocked and what it returned. Empty for states
    // that run no agent (terminal, guard-blocked, unavailable).
    pub tools: Vec<String>,
}

pub struct ChainOutcome {
    pub final_text: String,
    pub trace: Vec<ChainStep>,
}

fn state_preamble(state: &str, allowed: &[String], transcript: &str) -> String {
    format!(
        "You are executing step `{state}` of a deterministic chain. Allowed tool this step: {}. \
         If the user's goal needs this step, call the tool once with appropriate arguments and \
         report ONLY what it actually returned. If the goal does NOT need this step (e.g. a \
         read-only question reaching a write step), call nothing and reply exactly \
         `nothing to do this step`. Never invent ids, URLs, or contents, and never answer from \
         memory. If the tool returns an error or is refused, say so plainly. Call no other tool.\n\n\
         Context so far:\n{}",
        allowed.join(", "),
        if transcript.is_empty() {
            "(none yet)"
        } else {
            transcript
        }
    )
}

/// Run a validated chain to its terminal state through the agent-scoped, gate-enforced path.
/// `events` feeds the chat status UI (tool start/finish, and the approval proposals): the
/// caller owns the receiving end, so a chain run is observable end to end.
pub async fn run_chain(
    db: &Database,
    chain: &Chain,
    agent: &BuiltAgent,
    goal: &str,
    events: mpsc::UnboundedSender<crate::application::tools::ToolEvent>,
) -> Result<ChainOutcome, LlmError> {
    chain
        .validate()
        .map_err(|e| LlmError::Completion(format!("chain: {e}")))?;

    let llm = std::sync::Arc::new(LlmClient::from_db(db)?);
    let model = resolve_chat_model(&agent.model);
    // Web search is offered to a chain only when the agent's manifest governs `search_web` AND a
    // key is configured. No key, or no declaration, and the tool is simply not mounted.
    let web_key = agent
        .governs_web_search()
        .then(|| crate::application::web_search::exa_api_key(db))
        .filter(|k| !k.trim().is_empty());
    let backend = BackendClient::from_db(db).ok_or_else(|| {
        LlmError::NotConfigured("no backend configured".into())
    })?;
    // One session per connector the agent requires. The pool lives for the whole run: the tools
    // mounted below forward through its sinks.
    let pool =
        McpPool::connect_agent(db, &backend, &agent.slugs(), &agent.agent_id)
            .await
            .map_err(|e| LlmError::Completion(format!("mcp connect: {e}")))?;
    if let Some(dupe) = pool.duplicate_tools().first() {
        return Err(LlmError::Completion(format!(
            "two connectors serve the tool `{dupe}`: refusing to run rather than guess its owner"
        )));
    }

    // One base contract set, shared across states: budgets and read_before_write accumulate over
    // the whole run. The peers let the hook execute a user-EDITED payload deterministically, and
    // re-sync sheet headers, against the server that owns the tool.
    let base = ContractHook::with_contracts(events, agent.contract_entries())
        .with_schema(crate::application::connector_module::armed_schema_map(db))
        .with_peers(pool.peers_by_tool());

    let mut trace = Vec::new();
    let mut transcript = String::new();
    let mut name = chain.initial.clone();
    let start = std::time::Instant::now();

    // Loop bound is a hard ceiling independent of governance limits: a malformed chain cannot spin the device
    // (validation already rejects cycles, this is belt-and-suspenders).
    for _ in 0..=chain.states.len() {
        let state = chain.state(&name).expect("validated reachable state");
        // The run clock excludes time spent suspended on approval cards: `max_run_seconds`
        // bounds compute time, never the user's think-time.
        base.set_elapsed(
            start
                .elapsed()
                .as_secs()
                .saturating_sub(base.held_seconds()),
        );

        // Coarse: a write state reached before ANY bound resource was read is SKIPPED, never
        // entered - its tools are never mounted, so no write can happen (the gate still
        // enforces the precise per-resource rule). Skipping instead of killing the run lets a
        // read-only question flow through a linear chain to its answer.
        if matches!(state.guard, Some(Guard::ReadBeforeWrite))
            && !base.read_any()
        {
            trace.push(ChainStep {
                state: name.clone(),
                outcome:
                    "skipped: write state reached before the bound resource was read; no write performed"
                        .into(),
                tools: Vec::new(),
            });
            match state.on_done.clone() {
                Some(next) => {
                    name = next;
                    continue;
                }
                None => break,
            }
        }

        if state.terminal {
            // The goal rides along: without it the synthesis can only summarize the transcript
            // and tends to lead with incidental failures instead of answering the question.
            let answer_input =
                format!("User goal: {goal}\n\nChain context:\n{transcript}");
            let answer = llm
                .chat(ANSWER_PREAMBLE, &answer_input)
                .await
                .unwrap_or_else(|_| transcript.clone());
            trace.push(ChainStep {
                state: name.clone(),
                outcome: answer.clone(),
                tools: Vec::new(),
            });
            transcript = answer;
            break;
        }

        // Per connector, the tools this state allows. A connector contributing none is dropped:
        // mounting an empty tool list would still bind its sink for nothing.
        let avail: Vec<_> = pool
            .mounts()
            .into_iter()
            .map(|(tools, peer)| {
                (
                    tools
                        .into_iter()
                        .filter(|t| state.permits(t.name.as_ref()))
                        .collect::<Vec<_>>(),
                    peer,
                )
            })
            .filter(|(tools, _)| !tools.is_empty())
            .collect();
        if avail.is_empty() {
            trace.push(ChainStep {
                state: name.clone(),
                outcome: format!(
                    "unavailable: backend serves none of [{}] for this agent",
                    state.allowed_tools.join(", ")
                ),
                tools: Vec::new(),
            });
            break;
        }

        let hook = base.scoped_to(state.allowed_tools.clone(), name.clone());
        let preamble = state_preamble(&name, &state.allowed_tools, &transcript);
        let reply = llm
            .run_agent(
                model,
                &preamble,
                &format!("Goal: {goal}"),
                NotesTools::None,
                web_key.clone(),
                avail,
                hook,
                0.0,
                4,
            )
            .await?;
        // One completed state = one run step. Counting after the turn keeps `limits.max_steps` an inclusive
        // ceiling on states run (a step is a chain state; intra-state retries are bounded by max_tool_calls).
        base.bump_step();
        let tools = base.drain_events();
        transcript.push_str(&format!("\n[{name}] {reply}"));
        trace.push(ChainStep {
            state: name.clone(),
            outcome: reply,
            tools,
        });

        // A rejected/expired approval card ends the run's remaining write intents: jump
        // straight to the terminal answer so downstream write states never run.
        if base.aborted() {
            name = chain
                .states
                .iter()
                .find(|(_, s)| s.terminal)
                .map(|(n, _)| n.clone())
                .expect("validated chain reaches a terminal");
            continue;
        }

        name = state
            .on_done
            .clone()
            .expect("non-terminal validated to carry on_done");
    }

    // The hook re-syncs sheet headers against its own snapshot (it holds no Database);
    // persist whatever it refreshed so the next run starts from the live schema.
    if let Some(schema) = base.schema_snapshot() {
        crate::application::connector_module::store_schema_map(db, &schema);
    }

    Ok(ChainOutcome {
        final_text: transcript.trim().to_string(),
        trace,
    })
}
