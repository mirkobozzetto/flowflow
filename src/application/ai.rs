use crate::application::constants::{CHUNK_OVERLAP_WORDS, CHUNK_SIZE_WORDS};

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

/// Strip markdown noise (table pipes and separator rows, heading/quote/list markers) and collapse
/// whitespace, so a note-card preview reads as clean prose instead of raw `| a | --- |` syntax.
pub fn plain_preview(content: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // drop GFM table separator rows like `----|----|----`
        if trimmed.contains('-')
            && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        {
            continue;
        }
        // strip leading heading/quote/list markers, then turn table pipes into spaces
        let cleaned = trimmed
            .trim_start_matches(|c| {
                matches!(c, '#' | '>' | '-' | '*' | '+' | ' ')
            })
            .replace('|', " ");
        let cleaned = cleaned.trim();
        if !cleaned.is_empty() {
            parts.push(cleaned.to_string());
        }
    }
    parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_preview_strips_table_and_markers() {
        let md =
            "Date | Catégorie | Montant\n----|----|----\n01/10 | Alim | 50€";
        let p = plain_preview(md);
        assert!(!p.contains('|'), "pipes removed: {p}");
        assert!(!p.contains("----"), "separator row removed: {p}");
        assert!(p.contains("Date Catégorie Montant"));
        assert!(p.contains("01/10 Alim 50€"));
        // headings and list markers stripped
        assert_eq!(plain_preview("# Titre\n- point"), "Titre point");
    }

    #[test]
    fn char_prefix_is_utf8_safe() {
        let multibyte = "€".repeat(300);
        let cut = char_prefix(&multibyte, 200);
        assert_eq!(cut.chars().count(), 200);
        assert_eq!(char_prefix("café", 200), "café");
        assert_eq!(char_prefix("café", 3), "caf");
    }
}
