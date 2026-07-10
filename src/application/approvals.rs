// Propose-then-commit orchestration: the pending-approval registry the hook parks a held
// write in, the `decide()` entry point the card calls, and the human-readable view of the
// proposed payload. The gate stays pure (domain); the chat card stays dumb (ui); everything
// between the two lives here.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::time::Instant;
use uuid::Uuid;

use crate::domain::governance::{ConnectorManifest, Proposal};

/// How long a card waits before failing closed. Extended by `touch` while the user edits.
pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq)]
pub enum UserDecision {
    Approved,
    Edited(serde_json::Value),
    Rejected,
}

/// How the suspended call ended. `Expired` is the fail-closed default (timeout, closed
/// channel, dead run) - it behaves like a rejection everywhere downstream.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Approved,
    Edited(serde_json::Value),
    Rejected,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecideError {
    /// No live run is waiting on this proposal (timed out, decided, or the run died).
    Expired,
}

// What the card renders and what persists in the conversation. Field labels stay raw arg
// names; the UI translates the ones it knows. `raw_args` is the untouched proposal payload the
// card rebuilds an edit from: an Edited decision must carry the FULL args object, so the card
// substitutes the edited value fields into this rather than reconstructing from the rendered
// rows (which are stringified and reordered).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalView {
    pub id: String,
    pub tool: String,
    pub action: String,
    pub rows: Vec<(String, String)>,
    pub raw_args: serde_json::Value,
}

/// The persisted form of an approval card in the conversation (a message with role
/// `"proposal"`). Written at propose time as `Pending`, rewritten with the final status on
/// decision. Kept in the application layer so the reload contract is testable without the ui.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedProposal {
    pub view: ProposalView,
    pub status: String,
}

/// How a stored proposal renders when its conversation reloads. A still-`pending` row means its
/// in-memory run died (the app restarted): an in-memory run never survives, so it can never be
/// re-decided and fails closed to `Expired`. Unknown status strings collapse the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadStatus {
    Approved,
    Edited,
    Rejected,
    Expired,
}

/// Reconstruct a stored proposal into its view + reload status, or `None` if the content is not
/// a valid persisted proposal (dropped on load rather than rendered as garbage).
pub fn reload_proposal(content: &str) -> Option<(ProposalView, ReloadStatus)> {
    let parsed: PersistedProposal = serde_json::from_str(content).ok()?;
    let status = match parsed.status.as_str() {
        "approved" => ReloadStatus::Approved,
        "edited" => ReloadStatus::Edited,
        "rejected" => ReloadStatus::Rejected,
        _ => ReloadStatus::Expired,
    };
    Some((parsed.view, status))
}

/// The message roles the chat renders on reload. Any other role (an older or newer schema) is
/// dropped instead of being shown as a bot bubble with raw content.
pub fn is_renderable_role(role: &str) -> bool {
    matches!(role, "user" | "bot" | "proposal")
}

struct Pending {
    tx: Option<oneshot::Sender<UserDecision>>,
    deadline: Instant,
}

fn registry() -> &'static Mutex<HashMap<Uuid, Pending>> {
    static REGISTRY: OnceLock<Mutex<HashMap<Uuid, Pending>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock() -> std::sync::MutexGuard<'static, HashMap<Uuid, Pending>> {
    registry().lock().expect("approvals registry poisoned")
}

/// A registered proposal the hook awaits on. Dropping it (run cancelled, panic) removes the
/// registry entry, so a dead run can never leak a decidable card.
pub struct Registered {
    id: Uuid,
    rx: oneshot::Receiver<UserDecision>,
}

impl Drop for Registered {
    fn drop(&mut self) {
        lock().remove(&self.id);
    }
}

impl Registered {
    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// Park a held proposal and get the handle the hook will await. One entry per card.
pub fn register(timeout: Duration) -> Registered {
    register_with_id(Uuid::new_v4(), timeout)
}

/// Re-arm an existing card after a rejected edit: same id, fresh oneshot + deadline, so the
/// rendered card keeps its identity while the run waits again.
pub fn register_with_id(id: Uuid, timeout: Duration) -> Registered {
    let (tx, rx) = oneshot::channel();
    lock().insert(
        id,
        Pending {
            tx: Some(tx),
            deadline: Instant::now() + timeout,
        },
    );
    Registered { id, rx }
}

/// The card's single entry point: resolve the pending proposal with the user's decision.
pub fn decide(id: Uuid, decision: UserDecision) -> Result<(), DecideError> {
    let mut reg = lock();
    let pending = reg.get_mut(&id).ok_or(DecideError::Expired)?;
    let tx = pending.tx.take().ok_or(DecideError::Expired)?;
    tx.send(decision).map_err(|_| DecideError::Expired)
}

/// Keep a proposal alive while the user is mid-edit: pushes the deadline out to a full
/// timeout window from now. No-op on an unknown/decided proposal.
pub fn touch(id: Uuid, timeout: Duration) {
    if let Some(p) = lock().get_mut(&id) {
        p.deadline = Instant::now() + timeout;
    }
}

fn deadline_of(id: Uuid) -> Option<Instant> {
    lock().get(&id).map(|p| p.deadline)
}

/// Await the user's decision, fail-closed on the (touch-extensible) deadline. Consumes the
/// registration; the Drop guard clears the registry entry on every exit path.
pub async fn await_decision(mut registered: Registered) -> Outcome {
    loop {
        let Some(deadline) = deadline_of(registered.id) else {
            return Outcome::Expired;
        };
        tokio::select! {
            decision = &mut registered.rx => {
                return match decision {
                    Ok(UserDecision::Approved) => Outcome::Approved,
                    Ok(UserDecision::Edited(args)) => Outcome::Edited(args),
                    Ok(UserDecision::Rejected) => Outcome::Rejected,
                    Err(_) => Outcome::Expired,
                };
            }
            _ = tokio::time::sleep_until(deadline) => {
                // touch() may have moved the deadline while we slept; only a still-past
                // deadline expires the card.
                match deadline_of(registered.id) {
                    Some(d) if d > Instant::now() => continue,
                    _ => return Outcome::Expired,
                }
            }
        }
    }
}

/// Human-readable view of a proposal: connector-aware ordering for the fields the card
/// leads with, raw key/value fallback for everything else. Never invents data.
pub fn view_for(
    id: Uuid,
    proposal: &Proposal,
    conn: &ConnectorManifest,
) -> ProposalView {
    let action = conn
        .tool(&proposal.tool)
        .map(|t| format!("{:?}", t.action).to_lowercase())
        .unwrap_or_else(|| "write".into());

    let mut rows: Vec<(String, String)> = Vec::new();
    if let Some(obj) = proposal.args.as_object() {
        // Sheets-shaped fields first (the shipped connector); anything else in arg order.
        for key in ["spreadsheet_id", "sheet", "tab", "cell", "range", "value"]
        {
            if let Some(v) = obj.get(key) {
                rows.push((key.to_string(), scalar(v)));
            }
        }
        for (k, v) in obj {
            if !rows.iter().any(|(rk, _)| rk == k) {
                rows.push((k.clone(), scalar(v)));
            }
        }
    }

    ProposalView {
        id: id.to_string(),
        tool: proposal.tool.clone(),
        action,
        rows,
        raw_args: proposal.args.clone(),
    }
}

fn scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
