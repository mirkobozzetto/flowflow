use flowflow::application::ai::chunk_text;
use flowflow::application::constants::{CHUNK_OVERLAP_WORDS, CHUNK_SIZE_WORDS};

fn make_words(n: usize) -> String {
    (0..n)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn test_single_chunk() {
    let text = make_words(100);
    let chunks = chunk_text(&text);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn test_empty_input() {
    let chunks = chunk_text("");
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "");
}

#[test]
fn test_exact_chunk_size() {
    let text = make_words(CHUNK_SIZE_WORDS);
    let chunks = chunk_text(&text);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn test_just_above_chunk_size() {
    let text = make_words(CHUNK_SIZE_WORDS + 1);
    let chunks = chunk_text(&text);
    assert!(chunks.len() >= 2);
}

#[test]
fn test_multiple_chunks_with_overlap() {
    let total_words = 800;
    let text = make_words(total_words);
    let chunks = chunk_text(&text);

    assert!(
        chunks.len() >= 2,
        "expected multiple chunks, got {}",
        chunks.len()
    );

    for chunk in &chunks {
        let count = chunk.split_whitespace().count();
        assert!(
            count <= CHUNK_SIZE_WORDS,
            "chunk has {count} words, max is {CHUNK_SIZE_WORDS}"
        );
    }

    for pair in chunks.windows(2) {
        let prev_words: Vec<&str> = pair[0].split_whitespace().collect();
        let next_words: Vec<&str> = pair[1].split_whitespace().collect();

        let prev_tail = &prev_words[prev_words.len() - CHUNK_OVERLAP_WORDS..];
        let next_head = &next_words[..CHUNK_OVERLAP_WORDS];

        assert_eq!(
            prev_tail, next_head,
            "overlap mismatch between consecutive chunks"
        );
    }
}

#[test]
fn test_chunks_cover_full_text() {
    let total_words = 1000;
    let text = make_words(total_words);
    let chunks = chunk_text(&text);

    let first_word_of_text = "w0";
    let last_word_of_text = format!("w{}", total_words - 1);

    assert!(chunks.first().unwrap().starts_with(first_word_of_text));
    assert!(chunks.last().unwrap().ends_with(&last_word_of_text));
}
