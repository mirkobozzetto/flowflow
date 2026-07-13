// RFC 0023 T01 PoC: prove that a PromptHook returning Skip{reason: <filtered json>}
// feeds the LLM that json AS the tool result, and the model keeps working from it
// across multiple tool turns. The real tools below return a POISONED payload (leaked
// ids); the hook intercepts every call and substitutes the filtered payload. If the
// final answer names only armed sheets and never the leak markers, the seam holds.

use rig::agent::{PromptHook, ToolCallHookAction};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{CompletionModel, Prompt, ToolDefinition};
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug)]
struct Fail(String);
impl std::fmt::Display for Fail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Fail {}

// Tool 1: listing. Its REAL result leaks unarmed spreadsheets.
struct ListSpreadsheets;
#[derive(Deserialize)]
struct NoArgs {}
impl Tool for ListSpreadsheets {
    const NAME: &'static str = "list_spreadsheets";
    type Error = Fail;
    type Args = NoArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _p: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "List the user's spreadsheets.".into(),
            parameters: json!({"type": "object", "properties": {}}),
        }
    }
    async fn call(&self, _a: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(json!({"spreadsheets": [
            {"id": "armed-1", "name": "Clients"},
            {"id": "LEAK-salaries", "name": "LEAK Salaries"},
            {"id": "LEAK-passwords", "name": "LEAK Passwords"}
        ]}))
    }
}

// Tool 2: read. Real result also poisoned, to prove the SECOND turn is filtered too.
struct GetSpreadsheet;
#[derive(Deserialize)]
struct GetArgs {
    spreadsheet_id: String,
}
impl Tool for GetSpreadsheet {
    const NAME: &'static str = "get_spreadsheet";
    type Error = Fail;
    type Args = GetArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _p: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Read one spreadsheet's tabs by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"spreadsheet_id": {"type": "string"}},
                "required": ["spreadsheet_id"]
            }),
        }
    }
    async fn call(&self, a: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(json!({"id": a.spreadsheet_id, "sheets": ["LEAK-hidden-tab"]}))
    }
}

// The RFC 0023 seam: on_tool_call executes/filters ITSELF and skips the real call,
// returning the filtered json as the Skip reason (= the tool result the LLM sees).
#[derive(Clone)]
struct FilterHook;
impl<M: CompletionModel> PromptHook<M> for FilterHook {
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> ToolCallHookAction {
        eprintln!("[hook] intercept {tool_name} args={args}");
        let filtered = match tool_name {
            "list_spreadsheets" => {
                json!({"spreadsheets": [{"id": "armed-1", "name": "Clients"}],
                       "note": "scoped to armed resources"})
            }
            "get_spreadsheet" => {
                json!({"id": "armed-1", "sheets": ["Contacts"],
                       "note": "scoped to armed resources"})
            }
            _ => return ToolCallHookAction::Continue,
        };
        ToolCallHookAction::Skip {
            reason: filtered.to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let client =
        rig::providers::openai::Client::from_env().expect("OPENAI_API_KEY");
    let agent = client
        .agent("gpt-5.4-mini")
        .preamble(
            "You manage the user's spreadsheets. First call list_spreadsheets, \
             then call get_spreadsheet on every id you received, then answer.",
        )
        .tool(ListSpreadsheets)
        .tool(GetSpreadsheet)
        .build();

    let answer = agent
        .prompt(
            "Which spreadsheets do I have, and which tabs does each contain?",
        )
        .max_turns(6)
        .with_hook(FilterHook)
        .await
        .expect("prompt failed");

    println!("--- final answer ---\n{answer}\n---------------------");
    let leaked = answer.contains("LEAK");
    let armed = answer.contains("Clients") && answer.contains("Contacts");
    println!("leak_markers_present: {leaked}");
    println!("armed_content_used:   {armed}");
    if !leaked && armed {
        println!("POC PASS: Skip-reason is consumed as the tool result across turns.");
    } else {
        println!("POC FAIL");
        std::process::exit(1);
    }
}
