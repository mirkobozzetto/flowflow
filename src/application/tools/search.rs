use crate::application::constants::RAG_TOP_K;
use crate::application::tools::ToolFailure;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::vectordb::VectorStore;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct SearchNotes {
    pub llm: Arc<LlmClient>,
    /// The notes this run is allowed to see. `None` is a global chat; `Some(ids)`
    /// is a folder, a thread or an `@mention`.
    ///
    /// The tool used to search the whole corpus unconditionally, so a scoped chat
    /// leaked: the first retrieval respected the folder, then the agent's own
    /// re-search did not, and the extra notes reached the answer while the sources
    /// panel - built from the first retrieval only - never showed them.
    pub allowed_note_ids: Option<Arc<[String]>>,
}

impl SearchNotes {
    pub fn new(
        llm: Arc<LlmClient>,
        allowed_note_ids: Option<Arc<[String]>>,
    ) -> Self {
        Self {
            llm,
            allowed_note_ids,
        }
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
        // Return the hybrid hits as-is: a fixed cosine floor here wrongly drops keyword/proper-noun
        // matches (a search for "Jean" returned nothing, so the agent declared "no relevant note"
        // while the note was in its initial context). The agent judges relevance from the content.
        let results = store
            .hybrid_search(
                &args.query,
                vector,
                top_k,
                self.allowed_note_ids.as_deref(),
            )
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
