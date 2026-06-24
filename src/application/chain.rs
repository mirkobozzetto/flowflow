// The chain runtime: drives a validated FSM (`domain::orchestration`) over a connector's MCP tools. Rust
// owns the path - for each state it mounts only that state's tools, runs one gated agent turn, threads a
// transcript forward, enforces the read_before_write guard at the FSM seam, and composes a terminal answer.
// It reports a per-state trace, including states the backend serves no tool for, so gaps surface honestly.

use crate::application::error::LlmError;
use crate::application::tools::ContractHook;
use crate::domain::governance::{ConnectorManifest, Governance};
use crate::domain::orchestration::{Chain, Guard};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::mcp::McpRegistry;
use crate::infrastructure::persistence::Database;
use tokio::sync::mpsc;

const ANSWER_PREAMBLE: &str =
    "You are the final step of a tool chain. Compose a concise answer for the user from the \
     chain context below. Do not call any tool.";

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
         Call it once with appropriate arguments, then report ONLY what the tool actually returned. \
         Never invent ids, URLs, or contents, and never answer from memory. If the tool returns an error \
         or is refused, say so plainly. Call no other tool.\n\n\
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
pub async fn run_chain(
    db: &Database,
    chain: &Chain,
    gov: Governance,
    conn: ConnectorManifest,
    slug: &str,
    agent_id: &str,
    goal: &str,
) -> Result<ChainOutcome, LlmError> {
    chain
        .validate()
        .map_err(|e| LlmError::Completion(format!("chain: {e}")))?;

    let llm = LlmClient::from_db(db)?;
    let backend = BackendClient::from_db(db).ok_or_else(|| {
        LlmError::NotConfigured("no backend configured".into())
    })?;
    let reg = McpRegistry::connect_agent(db, &backend, slug, agent_id)
        .await
        .map_err(|e| LlmError::Completion(format!("mcp connect: {e}")))?;

    // One base contract, shared across states: budgets and read_before_write accumulate over the whole run.
    let (tx, _rx) = mpsc::unbounded_channel();
    let base = ContractHook::with_contract(tx, gov, conn);

    let mut trace = Vec::new();
    let mut transcript = String::new();
    let mut name = chain.initial.clone();
    let start = std::time::Instant::now();

    // Loop bound is a hard ceiling independent of governance limits: a malformed chain cannot spin the device
    // (validation already rejects cycles, this is belt-and-suspenders).
    for _ in 0..=chain.states.len() {
        let state = chain.state(&name).expect("validated reachable state");
        base.set_elapsed(start.elapsed().as_secs());

        if matches!(state.guard, Some(Guard::ReadBeforeWrite))
            && !base.read_bound()
        {
            trace.push(ChainStep {
                state: name.clone(),
                outcome: "blocked: entered a write state before the bound resource was read"
                    .into(),
                tools: Vec::new(),
            });
            break;
        }

        if state.terminal {
            let answer = llm
                .chat(ANSWER_PREAMBLE, &transcript)
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

        let avail: Vec<_> = reg
            .tools()
            .into_iter()
            .filter(|t| state.permits(t.name.as_ref()))
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
            .run_agent_over_tools(
                &preamble,
                &format!("Goal: {goal}"),
                avail,
                reg.peer(),
                hook,
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

        name = state
            .on_done
            .clone()
            .expect("non-terminal validated to carry on_done");
    }

    Ok(ChainOutcome {
        final_text: transcript.trim().to_string(),
        trace,
    })
}
