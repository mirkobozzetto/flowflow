use std::sync::{Arc, Mutex};
use std::time::Duration;

use flowflow::application::agent_native::{
    validate_native_capabilities, NativeCapability,
};
use flowflow::application::agent_native_execution::execute_native_with_confirmation;
use flowflow::application::approvals::{decide, DecideError, UserDecision};
use flowflow::application::tools::{ToolEvent, ToolFailure};
use flowflow::domain::governance::ToolPolicy;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::{json, Value};
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

fn launch(
    args: Value,
    timeout: Duration,
) -> (
    tokio::task::JoinHandle<Result<String, ToolFailure>>,
    mpsc::UnboundedReceiver<ToolEvent>,
    Arc<Mutex<Vec<String>>>,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool = RecordingCreate(calls.clone());
    let policy: ToolPolicy =
        serde_json::from_value(json!({"tool":"create_note",
        "mode":"read_write","approval":"require_approval"}))
        .unwrap();
    let plan = validate_native_capabilities(
        &[policy],
        &[NativeCapability::CreateNote],
        true,
    )
    .unwrap();
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        execute_native_with_confirmation(&tool, &plan, args, &tx, timeout).await
    });
    (task, rx, calls)
}

async fn proposal(rx: &mut mpsc::UnboundedReceiver<ToolEvent>) -> Uuid {
    match tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap()
    {
        ToolEvent::Proposal(view) => Uuid::parse_str(&view.id).unwrap(),
        event => panic!("unexpected {event:?}"),
    }
}

#[tokio::test]
async fn no_mutation_before_approval_and_exact_original_or_edited_args_execute_once(
) {
    for decision in [
        UserDecision::Approved,
        UserDecision::Edited(json!({"content":"edited"})),
    ] {
        let (task, mut rx, calls) =
            launch(json!({"content":"original"}), Duration::from_secs(2));
        let id = proposal(&mut rx).await;
        assert!(calls.lock().unwrap().is_empty());
        let expected = if matches!(decision, UserDecision::Approved) {
            "original"
        } else {
            "edited"
        };
        decide(id, decision).unwrap();
        assert_eq!(task.await.unwrap().unwrap(), expected);
        assert_eq!(*calls.lock().unwrap(), vec![expected.to_string()]);
        assert_eq!(
            decide(id, UserDecision::Approved),
            Err(DecideError::Expired)
        );
    }
}

#[tokio::test]
async fn rejection_invalid_edit_and_expiry_never_mutate() {
    for decision in [
        Some(UserDecision::Rejected),
        Some(UserDecision::Edited(json!({"content":4}))),
        None,
    ] {
        let timeout = if decision.is_none() {
            Duration::from_millis(10)
        } else {
            Duration::from_secs(2)
        };
        let (task, mut rx, calls) =
            launch(json!({"content":"original"}), timeout);
        let id = proposal(&mut rx).await;
        if let Some(decision) = decision {
            decide(id, decision).unwrap();
        }
        assert!(task.await.unwrap().is_err());
        assert!(calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn canceled_run_removes_pending_decision_and_never_mutates() {
    let (task, mut rx, calls) =
        launch(json!({"content":"original"}), Duration::from_secs(2));
    let id = proposal(&mut rx).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(
        decide(id, UserDecision::Approved),
        Err(DecideError::Expired)
    );
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_args_and_closed_surface_fail_without_mutation() {
    let (task, _rx, calls) =
        launch(json!({"content":4}), Duration::from_secs(2));
    assert!(task.await.unwrap().is_err());
    assert!(calls.lock().unwrap().is_empty());
    let (task, mut rx, calls) =
        launch(json!({"content":"original"}), Duration::from_secs(2));
    let id = proposal(&mut rx).await;
    drop(rx);
    decide(id, UserDecision::Approved).unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(calls.lock().unwrap().is_empty());
}
