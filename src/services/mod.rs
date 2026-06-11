pub mod ai;
pub mod audio;
pub mod backup;
pub mod constants;
pub mod embed;
pub mod error;
pub mod i18n;
pub mod llm;
pub mod rag;
pub mod reminders;
pub mod sync;
pub mod tools;
pub mod transcription;
pub mod vectordb;

pub use audio::{AudioRecorder, RecordingState};
pub use embed::{
    delete_attachment_embeddings, delete_note_embeddings, embed_attachment,
    embed_note,
};
pub use error::LlmError;
pub use llm::{LlmClient, Provider};
pub use rag::{RagResponse, RagSource};
pub use vectordb::{Chunk, SearchResult, VectorStore};
