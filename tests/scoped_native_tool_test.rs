use flowflow::application::agent_native::{
    validate_native_capabilities, NativeCapability,
};
use flowflow::application::agent_run::ScopedAgentRun;
use flowflow::application::approvals::{decide, UserDecision};
use flowflow::application::tools::scoped_native::ScopedNativeTool;
use flowflow::application::tools::{ToolEvent, ToolFailure};
use flowflow::domain::governance::{Limits, ToolPolicy};
use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolDyn},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
struct Recorder(Arc<Mutex<Vec<String>>>);
#[derive(Deserialize)]
struct Args {
    content: String,
}
impl Tool for Recorder {
    const NAME: &'static str = "create_note";
    type Args = Args;
    type Output = String;
    type Error = ToolFailure;
    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "recorder".into(),
            parameters: json!({"type":"object"}),
        }
    }
    async fn call(&self, args: Args) -> Result<String, ToolFailure> {
        self.0.lock().unwrap().push(args.content.clone());
        Ok(args.content)
    }
}
#[tokio::test]
async fn erased_mounted_tool_waits_for_confirmation_and_rechecks_quota() {
    let policy:ToolPolicy=serde_json::from_value(json!({"tool":"create_note","mode":"read_write","approval":"require_approval"})).unwrap();
    let plan = validate_native_capabilities(
        &[policy],
        &[NativeCapability::CreateNote],
        true,
    )
    .unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let run = Arc::new(ScopedAgentRun::new(
        tx,
        vec![],
        plan,
        Limits {
            max_tool_calls: Some(1),
            ..Default::default()
        },
    ));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let tool: Box<dyn ToolDyn> =
        Box::new(ScopedNativeTool::new(Recorder(writes.clone()), run));
    assert_eq!(tool.name(), "create_note");
    let task = tokio::spawn(async move {
        let result = tool.call(json!({"content":"original"}).to_string()).await;
        (tool, result)
    });
    let id = match rx.recv().await.unwrap() {
        ToolEvent::Proposal(view) => Uuid::parse_str(&view.id).unwrap(),
        other => panic!("{other:?}"),
    };
    assert!(writes.lock().unwrap().is_empty());
    decide(id, UserDecision::Edited(json!({"content":"approved edit"})))
        .unwrap();
    let (tool, result) = task.await.unwrap();
    assert!(result.is_ok());
    assert_eq!(*writes.lock().unwrap(), vec!["approved edit".to_string()]);
    assert!(tool
        .call(json!({"content":"second"}).to_string())
        .await
        .is_err());
    assert_eq!(writes.lock().unwrap().len(), 1);
}
