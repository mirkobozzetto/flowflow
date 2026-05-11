use crate::services::constants::{CHUNK_OVERLAP_WORDS, CHUNK_SIZE_WORDS};

pub fn chunk_text(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= CHUNK_SIZE_WORDS {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + CHUNK_SIZE_WORDS).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end >= words.len() {
            break;
        }
        start = end - CHUNK_OVERLAP_WORDS;
    }
    chunks
}
