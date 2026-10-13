pub mod audio;
pub mod backend;
pub mod chatgpt_auth;
// Anthropic forbids Claude.ai login in third-party apps: test builds only.
#[cfg(debug_assertions)]
pub mod claude_auth;
pub mod hermes;
pub mod llm;
pub mod mcp;
pub mod persistence;
pub mod platform;
pub mod sync;
pub mod token_store;
pub mod transcription;
pub mod vectordb;

pub use audio::{AudioRecorder, RecordingState};
pub use llm::{LlmClient, Provider};
pub use persistence::Database;
pub use vectordb::{Chunk, SearchResult, SourceType, VectorStore};
