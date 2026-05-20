use crate::db::Database;
use crate::models::NewTextNote;
use crate::services::embed::embed_note;
use crate::services::tools::ToolFailure;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Default)]
pub struct CreateNote;

impl CreateNote {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
pub struct CreateNoteArgs {
    pub title: Option<String>,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct CreateNoteResult {
    pub note_id: String,
    pub title: Option<String>,
}

impl Tool for CreateNote {
    const NAME: &'static str = "create_note";
    type Error = ToolFailure;
    type Args = CreateNoteArgs;
    type Output = CreateNoteResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a new text note in the user's library. \
                Use this when the user explicitly asks to save, write down, or remember something. \
                The note is automatically embedded for future search."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Optional short title. If omitted, will be auto-generated."
                    },
                    "content": {
                        "type": "string",
                        "description": "Full text content of the note."
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional tags (1-3 words each)."
                    }
                },
                "required": ["content"]
            }),
        }
    }

    async fn call(
        &self,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let db = Database::open()
            .map_err(|e| ToolFailure(format!("db open: {e}")))?;
        let new_note = NewTextNote {
            title: args.title.clone(),
            content: args.content.clone(),
            tags: args.tags.unwrap_or_default(),
        };
        let note = db
            .create_text_note(&new_note)
            .map_err(|e| ToolFailure(format!("create note: {e}")))?;
        let title_for_embed = note.title.clone().unwrap_or_default();
        embed_note(
            note.id.clone(),
            title_for_embed,
            note.content.clone(),
            note.tags.clone(),
            note.created_at.clone(),
        );
        Ok(CreateNoteResult {
            note_id: note.id,
            title: note.title,
        })
    }
}
