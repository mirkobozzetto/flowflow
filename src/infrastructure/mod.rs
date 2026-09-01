pub mod audio;
pub mod backend;
pub mod chatgpt_auth;
pub mod llm;
pub mod mcp;
pub mod persistence;
pub mod platform;
pub mod sync;
pub mod transcription;
pub mod vectordb;

pub use audio::{AudioRecorder, RecordingState};
pub use llm::{LlmClient, Provider};
pub use persistence::Database;
pub use vectordb::{Chunk, SearchResult, SourceType, VectorStore};
