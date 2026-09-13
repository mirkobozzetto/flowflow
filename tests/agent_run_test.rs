use std::sync::{Arc, Mutex};
use std::time::Duration;

use flowflow::application::agent_native::{
    validate_native_capabilities, NativeCapability,
};
use flowflow::application::approvals::{decide, DecideError, UserDecision};
use flowflow::application::tools::{ToolEvent, ToolFailure};
use flowflow::domain::governance::ToolPolicy;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

struct RecordingCreate(Arc<Mutex<Vec<String>>>);
#[derive(Deserialize)]
struct Args {
    content: String,
}
impl Tool for RecordingCreate {
    const NAME: &'static str = "create_note";
    type Error = ToolFailure;
    type Args = Args;
    type Output = String;
    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "test recorder".into(),
            parameters: json!({}),
        }
    }
    async fn call(&self, args: Args) -> Result<String, ToolFailure> {
        self.0.lock().unwrap().push(args.content.clone());
        Ok(args.content)
    }
}

use flowflow::application::agent_run::ScopedAgentRun;
use flowflow::domain::governance::{ConnectorManifest, Governance, Limits};
use rig::agent::{PromptHook, ToolCallHookAction};
type Model = rig::providers::openai::CompletionModel;

fn run(
    max_calls: u32,
    external: bool,
) -> (Arc<ScopedAgentRun>, mpsc::UnboundedReceiver<ToolEvent>) {
    let policy: ToolPolicy =
        serde_json::from_value(json!({"tool":"create_note",
        "mode":"read_write","approval":"require_approval"}))
        .unwrap();
    let native = validate_native_capabilities(
        &[policy],
        &[NativeCapability::CreateNote],
        true,
    )
    .unwrap();
    let entries = if external {
        let gov: Governance = serde_json::from_value(json!({"tools":[
            {"tool":"remote_read","mode":"read_only"}]}))
        .unwrap();
        let conn: ConnectorManifest = serde_json::from_value(json!({"connector":"remote","type":"reader",
            "server":"https://example.invalid","mcp_prefix":"remote_","provides":["read"],
            "tools":[{"tool":"remote_read","resource":"document","action":"read","risk":"read_only"}]})).unwrap();
        vec![("remote_".into(), gov, conn)]
    } else {
        vec![]
    };
    let (tx, rx) = mpsc::unbounded_channel();
    (
        Arc::new(ScopedAgentRun::new(
            tx,
            entries,
            native,
            Limits {
                max_tool_calls: Some(max_calls),
                max_steps: Some(3),
                max_run_seconds: Some(30),
            },
        )),
        rx,
    )
}

async fn remote(run: &ScopedAgentRun) -> ToolCallHookAction {
    PromptHook::<Model>::on_tool_call(
        run.external_hook(),
        "remote_read",
        None,
        "",
        "{}",
    )
    .await
}

async fn pending(rx: &mut mpsc::UnboundedReceiver<ToolEvent>) -> Uuid {
    loop {
        match tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ToolEvent::Proposal(view) => {
                return Uuid::parse_str(&view.id).unwrap()
            }
            _ => continue,
        }
    }
}

fn write(
    run: Arc<ScopedAgentRun>,
    calls: Arc<Mutex<Vec<String>>>,
) -> tokio::task::JoinHandle<Result<String, ToolFailure>> {
    tokio::spawn(async move {
        run.execute_native(
            &RecordingCreate(calls),
            json!({"content":"approved"}),
            Duration::from_secs(2),
        )
        .await
    })
}

#[tokio::test]
async fn native_charge_is_visible_to_external_calls() {
    let (run, mut rx) = run(1, true);
    let calls = Arc::default();
    let task = write(run.clone(), calls);
    decide(pending(&mut rx).await, UserDecision::Approved).unwrap();
    assert!(task.await.unwrap().is_ok());
    assert!(matches!(
        remote(&run).await,
        ToolCallHookAction::Skip { .. }
    ));
}

#[tokio::test]
async fn budget_race_during_approval_rejects_without_mutation() {
    let (run, mut rx) = run(1, true);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let task = write(run.clone(), calls.clone());
    let id = pending(&mut rx).await;
    assert!(matches!(remote(&run).await, ToolCallHookAction::Continue));
    decide(id, UserDecision::Approved).unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(calls.lock().unwrap().is_empty());
    assert!(run.aborted());
    let mut rejected_card = false;
    while let Ok(event) = rx.try_recv() {
        if let ToolEvent::ProposalResolved { status, .. } = event {
            rejected_card = status
                == flowflow::application::tools::ProposalStatus::Rejected;
        }
    }
    assert!(rejected_card);
}

#[tokio::test]
async fn rejection_and_cancellation_stop_later_external_and_native_calls() {
    for cancel in [false, true] {
        let (run, mut rx) = run(5, true);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let task = write(run.clone(), calls.clone());
        let id = pending(&mut rx).await;
        if cancel {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            decide(id, UserDecision::Rejected).unwrap();
            assert!(task.await.unwrap().is_err());
        }
        assert!(run.aborted());
        assert!(matches!(
            remote(&run).await,
            ToolCallHookAction::Skip { .. }
        ));
        assert!(write(run.clone(), calls.clone()).await.unwrap().is_err());
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(
            decide(id, UserDecision::Approved),
            Err(DecideError::Expired)
        );
    }
}

#[tokio::test]
async fn native_only_runs_enforce_step_and_time_limits_before_proposing() {
    for progress in [(3, 0), (0, 30)] {
        let (run, mut rx) = run(5, false);
        run.set_progress(progress.0, progress.1);
        run.set_progress(0, 0);
        assert!(write(run, Arc::default()).await.unwrap().is_err());
        assert!(rx.try_recv().is_err());
    }
}
