// The device-side agent directory: list the published agents this account is entitled to
// (GET /v1/agents) and install/remove any of them through the signed-package pipeline.
// Discovery lives here; running an installed agent stays in `connector_module`.

use crate::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use crate::infrastructure::backend::{AgentSummary, BackendClient};
use crate::infrastructure::persistence::Database;

/// One row of the directory as the Settings list renders it.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub installed: bool,
}

/// The entitled agents, each flagged with its local install state.
pub async fn list_agents(db: &Database) -> Result<Vec<AgentEntry>, String> {
    let backend = BackendClient::from_db(db)
        .ok_or("no backend configured".to_string())?;
    let summaries = backend
        .list_agents(db)
        .await
        .map_err(|e| format!("list agents: {e}"))?;
    let installed: Vec<String> = db
        .list_installed_agents()
        .into_iter()
        .map(|a| a.id)
        .collect();
    Ok(mark_installed(summaries, &installed))
}

/// Pure join of the backend list with the locally pinned ids, kept separate so the
/// mapping is testable offline.
pub fn mark_installed(
    summaries: Vec<AgentSummary>,
    installed_ids: &[String],
) -> Vec<AgentEntry> {
    summaries
        .into_iter()
        .map(|s| AgentEntry {
            installed: installed_ids.iter().any(|i| i == &s.id),
            id: s.id,
            name: s.display_name,
            alias: s.alias,
        })
        .collect()
}

/// Install (pin) one agent by id: check the kill switch, then fetch + verify + pin the
/// signed package if it is not already pinned. A pinned row whose version was revoked is
/// dropped and the install refused - the cross-device kill switch.
pub async fn ensure_installed(
    db: &Database,
    agent_id: &str,
) -> Result<(), String> {
    let backend = BackendClient::from_db(db)
        .ok_or("no backend configured".to_string())?;

    let revoked = backend
        .fetch_revocations(db)
        .await
        .map_err(|e| format!("revocation check: {e}"))?;

    if let Some(existing) = db.get_installed_agent(agent_id) {
        if revoked
            .iter()
            .any(|r| r.id == agent_id && r.version == existing.version)
        {
            db.uninstall_agent(agent_id)?;
            return Err(format!("agent `{agent_id}` was revoked"));
        }
        return Ok(());
    }

    let package = backend
        .fetch_agent_package(db, agent_id)
        .await
        .map_err(|e| format!("fetch agent package: {e}"))?;
    let verified = verify_package(&package, ADMIN_PUBKEY)
        .map_err(|e| format!("verify agent package: {e}"))?;
    db.install_agent(&verified)
}

/// Drop the pinned row. Bindings and notes are untouched; reinstalling re-fetches the
/// current signed package.
pub fn uninstall(db: &Database, agent_id: &str) -> Result<(), String> {
    db.uninstall_agent(agent_id)
}
