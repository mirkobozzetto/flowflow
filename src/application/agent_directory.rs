// The device-side agent directory: list the published agents this account is entitled to
// (GET /v1/agents) and install/remove any of them through the signed-package pipeline.
// Discovery lives here; running an installed agent stays in `connector_module`.

use crate::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use crate::infrastructure::backend::{AgentSummary, BackendClient};
use crate::infrastructure::persistence::Database;

/// One row of the directory as the Settings list renders it. `stale` marks an installed row
/// the served (entitlement-filtered) directory does not know: a ghost id, or an agent gone
/// hidden/unpublished. It renders as a removable card - NEVER auto-uninstalled, because the
/// row carries the user's arm-time binding (bound_json) and a silent sweep would lose it.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub installed: bool,
    pub stale: bool,
}

/// The entitled agents flagged with their install state, plus the installed rows the
/// directory no longer serves (stale), so every local install stays visible and removable.
pub async fn list_agents(db: &Database) -> Result<Vec<AgentEntry>, String> {
    let backend = BackendClient::from_db(db)
        .ok_or("no backend configured".to_string())?;
    let summaries = backend
        .list_agents(db)
        .await
        .map_err(|e| format!("list agents: {e}"))?;
    let installed = db.list_installed_agents();
    let installed_ids: Vec<String> =
        installed.iter().map(|a| a.id.clone()).collect();
    let mut entries = mark_installed(summaries, &installed_ids);
    entries.extend(stale_entries(&entries, &installed));
    Ok(entries)
}

// Installed rows absent from the served list, rendered from their pinned manifest (name,
// alias parsed as data; the id itself is the fallback name).
fn stale_entries(
    served: &[AgentEntry],
    installed: &[crate::domain::agent_manifest::InstalledAgent],
) -> Vec<AgentEntry> {
    installed
        .iter()
        .filter(|row| !served.iter().any(|e| e.id == row.id))
        .map(|row| {
            let manifest = crate::domain::agent_manifest::parse_manifest(
                &row.manifest_json,
            )
            .ok();
            AgentEntry {
                id: row.id.clone(),
                name: manifest
                    .as_ref()
                    .map(|m| m.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| row.id.clone()),
                alias: manifest.map(|m| m.alias).unwrap_or_default(),
                installed: true,
                stale: true,
            }
        })
        .collect()
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
            stale: false,
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
            stash_binding(db, agent_id);
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
    db.install_agent(&verified)?;
    restore_binding(db, agent_id);

    // Connector pins ride the same install moment (RFC 0016): revocations first (a revoked slug
    // must not survive), then fetch + verify + pin what this agent requires. Best-effort: a pin
    // failure surfaces at arm ("no connector pinned"), never blocks the agent install itself.
    if let Err(e) =
        crate::application::connector_pins::apply_revocations(db, &backend)
            .await
    {
        eprintln!("[connectors] revocation sweep failed: {e}");
    }
    if let Err(e) =
        crate::application::connector_pins::refresh_pins(db, &backend).await
    {
        eprintln!("[connectors] pin refresh failed: {e}");
    }
    Ok(())
}

// The arm-time binding lives on the pinned row, so dropping the row would silently lose the
// user's armed sheets. Stash it in settings across the uninstall; the next install restores it.
fn binding_stash_key(agent_id: &str) -> String {
    format!("stashed_binding:{agent_id}")
}

fn stash_binding(db: &Database, agent_id: &str) {
    if let Some(bound) = db.get_agent_binding(agent_id) {
        let _ =
            db.set_setting(&binding_stash_key(agent_id), &bound.to_string());
    }
}

fn restore_binding(db: &Database, agent_id: &str) {
    let key = binding_stash_key(agent_id);
    let Some(stashed) = db.get_setting(&key) else {
        return;
    };
    if db.get_agent_binding(agent_id).is_none() {
        let _ = db.set_agent_binding(agent_id, Some(&stashed));
    }
    let _ = db.delete_setting(&key);
}

/// Test seam: the restore half without the network install around it.
pub fn restore_binding_for_test(db: &Database, agent_id: &str) {
    restore_binding(db, agent_id)
}

/// Drop the pinned row, stashing the arm-time binding so a reinstall restores the user's
/// armed sheets. Notes are untouched; reinstalling re-fetches the current signed package.
pub fn uninstall(db: &Database, agent_id: &str) -> Result<(), String> {
    stash_binding(db, agent_id);
    db.uninstall_agent(agent_id)
}
