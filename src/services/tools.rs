use crate::db::Database;
use crate::models::NewTextNote;
use crate::services::constants::{
    CHAT_MODEL, RAG_TOP_K, SUMMARIZE_FOLDER_PROMPT,
};
use crate::services::embed::embed_note;
use crate::services::llm::LlmClient;
use crate::services::vectordb::VectorStore;
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, Prompt, ToolDefinition};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum ToolEvent {
    Started(String),
    Finished(String),
}

#[derive(Clone)]
pub struct ToolStatusHook {
    tx: mpsc::UnboundedSender<ToolEvent>,
}

impl ToolStatusHook {
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self { tx }
    }
}

impl<M: CompletionModel> PromptHook<M> for ToolStatusHook {
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
    ) -> ToolCallHookAction {
        let _ = self.tx.send(ToolEvent::Started(tool_name.to_string()));
        ToolCallHookAction::Continue
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        _result: &str,
    ) -> HookAction {
        let _ = self.tx.send(ToolEvent::Finished(tool_name.to_string()));
        HookAction::cont()
    }
}

#[derive(Debug)]
pub struct ToolFailure(pub String);

impl std::fmt::Display for ToolFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolFailure {}

#[derive(Clone)]
pub struct SearchNotes {
    pub llm: Arc<LlmClient>,
}

impl SearchNotes {
    pub fn new(llm: Arc<LlmClient>) -> Self {
        Self { llm }
    }
}

#[derive(Deserialize)]
pub struct SearchNotesArgs {
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchNotesHit {
    pub note_id: String,
    pub title: String,
    pub excerpt: String,
    pub distance: f32,
}

impl Tool for SearchNotes {
    const NAME: &'static str = "search_notes";
    type Error = ToolFailure;
    type Args = SearchNotesArgs;
    type Output = Vec<SearchNotesHit>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the user's notes using semantic similarity. \
                Use this when the user asks about a topic, concept, or keyword \
                that may be present in their notes. Returns up to `top_k` matching excerpts."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural-language search query (use the same language as the user)."
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Maximum number of results to return. Defaults to 5."
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(
        &self,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let top_k = args.top_k.unwrap_or(RAG_TOP_K);
        let vector = self
            .llm
            .embed(&args.query)
            .await
            .map_err(|e| ToolFailure(format!("embed: {e}")))?;
        let store = VectorStore::open()
            .await
            .map_err(|e| ToolFailure(format!("vectordb open: {e}")))?;
        let results = store
            .search(vector, top_k)
            .await
            .map_err(|e| ToolFailure(format!("vectordb search: {e}")))?;
        Ok(results
            .into_iter()
            .map(|r| SearchNotesHit {
                note_id: r.note_id,
                title: r.title,
                excerpt: r.chunk_text,
                distance: r.distance,
            })
            .collect())
    }
}

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
        embed_note(note.id.clone(), title_for_embed, note.content.clone());
        Ok(CreateNoteResult {
            note_id: note.id,
            title: note.title,
        })
    }
}

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

pub async fn prompt_agent_with_tools(
    llm: Arc<LlmClient>,
    preamble: &str,
    user_message: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<String, crate::services::error::LlmError> {
    let agent = llm
        .inner()
        .agent(CHAT_MODEL)
        .preamble(preamble)
        .temperature(0.3)
        .tool(SearchNotes::new(llm.clone()))
        .tool(CreateNote::new())
        .tool(SummarizeFolder::new(llm.clone()))
        .build();
    let request = agent.prompt(user_message).max_turns(4);
    let result = if let Some(tx) = status_tx {
        request.with_hook(ToolStatusHook::new(tx)).await
    } else {
        request.await
    };
    result.map_err(|e| {
        crate::services::error::LlmError::Completion(e.to_string())
    })
}
