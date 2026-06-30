use crate::application::constants::{RAG_RELEVANCE_FLOOR, RAG_TOP_K};
use crate::application::rag::floor_filter;
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
            .hybrid_search(&args.query, vector, top_k, None)
            .await
            .map_err(|e| ToolFailure(format!("vectordb search: {e}")))?;
        // Same absolute floor as the main RAG path so a self-retrieving agent cannot
        // re-inject un-floored, irrelevant chunks and route around the grounding gate.
        let results = floor_filter(&results, RAG_RELEVANCE_FLOOR);
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
