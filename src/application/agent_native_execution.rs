//! Opt-in per-call native authorization. Legacy tools are not wrapped here.
//! The caller owns run-wide budgets and must never mount the raw tool alongside
//! this path. Schema-2 mounting remains disabled until that integration exists.

use std::time::Duration;

use rig::tool::Tool;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::application::agent_native::{
    NativeCapability, NativeCapabilityPlan,
};
use crate::application::approvals::{self, Outcome, ProposalView};
use crate::application::tools::{ProposalStatus, ToolEvent, ToolFailure};

/// Invoke the actual typed tool only after its declared capability is admitted
/// and, for writes, a live decision confirms these exact arguments. No detached
/// mutation task is spawned: dropping this future drops the pending registration.
pub async fn execute_native_with_confirmation<T: Tool>(
    tool: &T,
    plan: &NativeCapabilityPlan,
    args: Value,
    events: &UnboundedSender<ToolEvent>,
    timeout: Duration,
) -> Result<T::Output, ToolFailure> {
    execute_native_guarded(tool, plan, args, events, timeout, |_| Ok(()), || ())
        .await
}

/// The shared-run adapter checks before proposing and atomically charges just
/// before execution. Approval waiting must not reserve a reusable execution slot.
pub(crate) async fn execute_native_guarded<T: Tool, W>(
    tool: &T,
    plan: &NativeCapabilityPlan,
    args: Value,
    events: &UnboundedSender<ToolEvent>,
    timeout: Duration,
    mut check_run: impl FnMut(bool) -> Result<(), ToolFailure>,
    on_wait: impl FnOnce() -> W,
) -> Result<T::Output, ToolFailure> {
    if !plan.policies().iter().any(|policy| policy.tool == T::NAME) {
        return Err(ToolFailure(format!(
            "native tool `{}` was not admitted",
            T::NAME
        )));
    }
    let capability = NativeCapability::from_name(T::NAME)
        .ok_or_else(|| ToolFailure("unknown native capability".into()))?;
    if !args.is_object() {
        return Err(ToolFailure(
            "native tool arguments must be an object".into(),
        ));
    }
    let mut typed_args: T::Args = serde_json::from_value(args.clone())
        .map_err(|error| {
            ToolFailure(format!("invalid native tool arguments: {error}"))
        })?;
    check_run(false)?;
    if capability.action().is_write() {
        let registered = approvals::register(timeout);
        let id = registered.id();
        let view = ProposalView {
            id: id.to_string(),
            tool: T::NAME.into(),
            action: "create".into(),
            rows: args
                .as_object()
                .expect("object checked")
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    )
                })
                .collect(),
            raw_args: args,
        };
        let waiting = on_wait();
        events.send(ToolEvent::Proposal(view)).map_err(|_| {
            ToolFailure("native write has no live decision channel".into())
        })?;
        let decision = approvals::await_decision(registered).await;
        drop(waiting);
        let (mut status, mut refusal) = match decision {
            Outcome::Approved => (ProposalStatus::Approved, None),
            Outcome::Edited(edited) => {
                if !edited.is_object() {
                    (
                        ProposalStatus::Rejected,
                        Some("edited arguments must be an object".into()),
                    )
                } else {
                    match serde_json::from_value(edited) {
                        Ok(value) => {
                            typed_args = value;
                            (ProposalStatus::Edited, None)
                        }
                        Err(error) => (
                            ProposalStatus::Rejected,
                            Some(format!("invalid edited arguments: {error}")),
                        ),
                    }
                }
            }
            Outcome::Rejected => (
                ProposalStatus::Rejected,
                Some("native write rejected".into()),
            ),
            Outcome::Expired => (
                ProposalStatus::Expired,
                Some("native write approval expired".into()),
            ),
        };
        if refusal.is_none() {
            if let Err(error) = check_run(true) {
                status = ProposalStatus::Rejected;
                refusal = Some(error.to_string());
            }
        }
        // If the surface disappeared after the decision, fail closed instead of
        // writing after the user left the approval flow.
        events
            .send(ToolEvent::ProposalResolved {
                id: id.to_string(),
                status,
            })
            .map_err(|_| {
                ToolFailure(
                    "native decision channel closed before execution".into(),
                )
            })?;
        if let Some(reason) = refusal {
            return Err(ToolFailure(reason));
        }
    } else {
        check_run(true)?;
    }
    tool.call(typed_args)
        .await
        .map_err(|error| ToolFailure(error.to_string()))
}
