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

pub fn char_prefix(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_prefix_is_utf8_safe() {
        let multibyte = "€".repeat(300);
        let cut = char_prefix(&multibyte, 200);
        assert_eq!(cut.chars().count(), 200);
        assert_eq!(char_prefix("café", 200), "café");
        assert_eq!(char_prefix("café", 3), "caf");
    }
}
