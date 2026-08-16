// The web search tool an agent can call mid-run. Same Exa call the chat RAG already makes
// (`web_search::exa_search`), exposed as a tool instead of a retrieval step, so a chain can search
// before it acts. Read-only: it fetches nothing and writes nothing.

use crate::application::tools::ToolFailure;
use crate::application::web_search::exa_search;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

// Kept small on purpose: these results ride in the model's context, and a chain state may call
// the tool several times.
const MAX_RESULTS: usize = 5;

#[derive(Clone)]
pub struct SearchWeb {
    api_key: String,
}

impl SearchWeb {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Deserialize)]
pub struct SearchWebArgs {
    pub query: String,
}

#[derive(Serialize)]
pub struct SearchWebHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_date: Option<String>,
}

impl Tool for SearchWeb {
    const NAME: &'static str = "search_web";
    type Error = ToolFailure;
    type Args = SearchWebArgs;
    type Output = Vec<SearchWebHit>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the public web for current information: news, prices, \
                job or mission listings, company and people facts. Use it when the answer \
                is not in the user's notes and depends on what is true today. Returns a \
                title, URL and excerpt per result."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What to search for, in natural language."
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
        if self.api_key.trim().is_empty() {
            return Err(ToolFailure("no Exa API key configured".into()));
        }
        let hits = exa_search(&args.query, &self.api_key).await;
        Ok(hits
            .into_iter()
            .take(MAX_RESULTS)
            .map(|r| SearchWebHit {
                title: r.title,
                url: r.url,
                snippet: r.snippet,
                published_date: r.published_date,
            })
            .collect())
    }
}
