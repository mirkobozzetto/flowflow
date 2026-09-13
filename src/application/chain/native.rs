//! Installed schema-2 dispatch across validated native and external owners.
//! Legacy chains remain on their original path.
use super::*;
use crate::application::agent_native::NativeCapability;
use crate::infrastructure::{backend::BackendClient, mcp::McpPool};
use std::sync::Arc;

pub(crate) async fn run_installed_native_chain(
    db: &Database,
    agent_id: &str,
    chain_name: &str,
    goal: &str,
    events: mpsc::UnboundedSender<crate::application::tools::ToolEvent>,
) -> Result<ChainOutcome, String> {
    run_with_client(db, agent_id, chain_name, goal, events, |db| {
        LlmClient::from_db(db)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    })
    .await
}

pub(crate) async fn run_with_client(
    db: &Database,
    agent_id: &str,
    chain_name: &str,
    goal: &str,
    events: mpsc::UnboundedSender<crate::application::tools::ToolEvent>,
    client: impl FnOnce(&Database) -> Result<Arc<LlmClient>, String>,
) -> Result<ChainOutcome, String> {
    let (manifest, identity, pin_check) =
        crate::application::agent_selections::context(db, agent_id)?;
    let (bindings, resolved) =
        crate::application::agent_selections::load(db, &identity, &manifest)?;
    let check = crate::application::agent_selections::snapshot_check(
        db, &identity, &bindings, pin_check,
    )?;
    let chain = manifest
        .execution
        .orchestration
        .chains
        .get(chain_name)
        .ok_or_else(|| format!("manifest has no `{chain_name}` chain"))?;
    chain.validate().map_err(|error| error.to_string())?;
    // The native implementations open the application store themselves. Never
    // let a pin loaded from a different database authorize those tools.
    let store = db
        .conn()
        .path()
        .map(std::path::PathBuf::from)
        .ok_or("database path unavailable")?;
    if store.canonicalize().map_err(|e| e.to_string())?
        != crate::infrastructure::persistence::db_path()
            .canonicalize()
            .map_err(|e| e.to_string())?
    {
        return Err(
            "native tools and installed package use different databases".into(),
        );
    }
    let web_key = crate::application::web_search::exa_api_key(db);
    let web_key = (!web_key.trim().is_empty()).then_some(web_key);
    let mut available = vec![
        NativeCapability::SearchNotes,
        NativeCapability::CreateNote,
        NativeCapability::SummarizeFolder,
        NativeCapability::ScheduleReminder,
    ];
    if web_key.is_some() {
        available.push(NativeCapability::SearchWeb);
    }
    let requirements: Vec<_> = manifest
        .execution
        .required_connectors
        .iter()
        .map(|requirement| {
            crate::domain::capability_contract::CapabilityRequirement {
                key: requirement.key.clone(),
                connector_type: requirement.connector_type.clone(),
                capabilities: requirement.capabilities.clone(),
            }
        })
        .collect();
    let base = crate::application::agent_execution::assemble_scoped_run(
        crate::application::agent_execution::ScopedExecutionInput {
            identity: &identity,
            governance: &manifest.execution.governance,
            requirements: &requirements,
            resolved: &resolved,
            bindings: &bindings,
            native_tools: &manifest.execution.native_tools,
            available_native: &available,
        },
        events,
    )?;
    let pool = if resolved.is_empty() {
        None
    } else {
        let backend =
            BackendClient::from_db(db).ok_or("backend is not configured")?;
        Some(
            McpPool::connect_scoped(db, &backend, &identity, &resolved)
                .await
                .map_err(|error| error.to_string())?,
        )
    };
    let peers = pool
        .as_ref()
        .map(McpPool::peers_by_tool)
        .unwrap_or_default();
    let run = Arc::new(
        base.with_external_context(
            peers,
            crate::application::connector_module::armed_schema_map(db),
        )
        .with_admission_check(check.clone()),
    );
    let llm = client(db)?;
    let mut trace: Vec<ChainStep> = Vec::new();
    let mut transcript = String::new();
    let mut name = chain.initial.clone();
    let mut steps = 0;
    let mut skipped = false;
    for _ in 0..=chain.states.len() {
        check()?;
        run.set_progress(steps, run.elapsed_seconds());
        let state = chain.state(&name).expect("validated state");
        if state.terminal {
            let no_op =
                !skipped && trace.iter().all(|step| step.tools.is_empty());
            let preamble = with_mission(
                &manifest.system_prompt,
                if no_op {
                    NO_OP_PREAMBLE
                } else {
                    ANSWER_PREAMBLE
                }
                .into(),
            );
            let answer = llm
                .chat(
                    &preamble,
                    &format!(
                        "User goal: {goal}\n\nChain context:\n{transcript}"
                    ),
                )
                .await
                .map_err(|error| format!("terminal synthesis: {error}"))?;
            trace.push(ChainStep {
                state: name,
                outcome: answer.clone(),
                tools: Vec::new(),
            });
            return Ok(ChainOutcome {
                final_text: answer,
                trace,
            });
        }
        if matches!(state.guard, Some(Guard::ReadBeforeWrite))
            && !run.external_hook().admitted_external_read()
        {
            skipped = true;
            trace.push(ChainStep { state: name.clone(), outcome:
                "skipped: no external read was admitted; no write performed".into(), tools: Vec::new() });
        } else {
            let preamble = with_mission(
                &manifest.system_prompt,
                state_preamble(&name, &state.allowed_tools, &transcript),
            );
            let reply = llm
                .run_scoped(
                    resolve_chat_model(&manifest.model),
                    &preamble,
                    &format!("Goal: {goal}"),
                    run.clone(),
                    &state.allowed_tools,
                    web_key.clone(),
                    f64::from(manifest.temperature.unwrap_or(0.0)),
                    pool.as_ref()
                        .map(|pool| {
                            pool.mounts()
                                .into_iter()
                                .filter_map(|(tools, peer)| {
                                    let tools: Vec<_> = tools
                                        .into_iter()
                                        .filter(|tool| {
                                            state.allowed_tools.iter().any(
                                                |name| {
                                                    name == tool.name.as_ref()
                                                },
                                            )
                                        })
                                        .collect();
                                    (!tools.is_empty()).then_some((tools, peer))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                )
                .await
                .map_err(|e| e.to_string())?;
            steps += 1;
            let mut tools = run.external_hook().drain_events();
            tools.extend(run.drain_native_events());
            transcript.push_str(&format!("\n[{name}] {reply}"));
            if !tools.is_empty() {
                transcript.push_str("\nRecorded tool outcomes:\n");
                transcript.push_str(&tools.join("\n"));
            }
            trace.push(ChainStep {
                state: name.clone(),
                outcome: reply,
                tools,
            });
        }
        name = if run.aborted() {
            chain
                .states
                .iter()
                .find(|(_, state)| state.terminal)
                .map(|(name, _)| name.clone())
                .expect("validated terminal")
        } else {
            state.on_done.clone().expect("validated transition")
        };
    }
    Err("native chain exceeded its validated state limit".into())
}
