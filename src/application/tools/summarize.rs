use crate::application::constants::SUMMARIZE_FOLDER_PROMPT;
use crate::application::tools::ToolFailure;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct SummarizeFolder {
    pub llm: Arc<LlmClient>,
}

impl SummarizeFolder {
    pub fn new(llm: Arc<LlmClient>) -> Self {
        Self { llm }
    }
}

#[derive(Deserialize)]
pub struct SummarizeFolderArgs {
    pub folder_name: String,
    pub max_notes: Option<usize>,
}

#[derive(Serialize)]
pub struct SummarizeFolderResult {
    pub folder_name: String,
    pub note_count: usize,
    pub summary: String,
}

impl Tool for SummarizeFolder {
    const NAME: &'static str = "summarize_folder";
    type Error = ToolFailure;
    type Args = SummarizeFolderArgs;
    type Output = SummarizeFolderResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Summarize all notes inside a folder by its name. \
                Use this when the user asks for a recap, summary, or overview of a folder."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "folder_name": {
                        "type": "string",
                        "description": "Exact or close folder name."
                    },
                    "max_notes": {
                        "type": "integer",
                        "description": "Maximum number of notes to include in the summary. Defaults to 20."
                    }
                },
                "required": ["folder_name"]
            }),
        }
    }

    async fn call(
        &self,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let db = Database::open()
            .map_err(|e| ToolFailure(format!("db open: {e}")))?;
        let folders = db
            .list_all_folders()
            .map_err(|e| ToolFailure(format!("list folders: {e}")))?;
        let needle = args.folder_name.to_lowercase();
        let folder = folders
            .into_iter()
            .find(|f| f.name.to_lowercase() == needle)
            .or_else(|| {
                db.list_all_folders()
                    .ok()?
                    .into_iter()
                    .find(|f| f.name.to_lowercase().contains(&needle))
            })
            .ok_or_else(|| {
                ToolFailure(format!("folder not found: {}", args.folder_name))
            })?;
        let notes = db
            .list_notes_in_folder(&folder.id)
            .map_err(|e| ToolFailure(format!("list notes: {e}")))?;
        if notes.is_empty() {
            return Ok(SummarizeFolderResult {
                folder_name: folder.name,
                note_count: 0,
                summary: "This folder is empty.".to_string(),
            });
        }
        let limit = args.max_notes.unwrap_or(20).min(notes.len());
        let total = notes.len();
        let body = notes
            .into_iter()
            .take(limit)
            .map(|n| {
                let title = n.title.unwrap_or_else(|| "Untitled".to_string());
                format!("## {title}\n{}\n", n.content)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let user_msg = format!(
            "Folder: {}\nTotal notes in folder: {total}\n\n{body}",
            folder.name
        );
        let summary = self
            .llm
            .chat(SUMMARIZE_FOLDER_PROMPT, &user_msg)
            .await
            .map_err(|e| ToolFailure(format!("chat: {e}")))?;
        Ok(SummarizeFolderResult {
            folder_name: folder.name,
            note_count: total,
            summary,
        })
    }
}
