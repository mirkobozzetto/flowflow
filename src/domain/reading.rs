use crate::domain::Word;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// A part of one recording on a single topic. It opens at the first word of a
/// pause paragraph, so its heading lands right above that paragraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub start_word: usize,
    pub title: String,
    pub summary: String,
}

/// A silence at least this long between two words starts a new paragraph.
/// Measured on three real meetings: 72 to 82 words per paragraph.
pub const PAUSE_PARAGRAPH_MS: u32 = 2000;

/// A paragraph shorter than this is only matched where the previous one ended:
/// a few words ("Yeah. Okay.") recur anywhere, and a match found further down
/// would skip every real paragraph in between.
const MIN_WORDS_TO_SEARCH: usize = 8;

/// One piece of a note body, in reading order.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    /// Text that is not a transcript paragraph as spoken: typed text, or a
    /// paragraph corrected by hand.
    Text(String),
    /// A paragraph found verbatim in the body: `words` indexes the words of
    /// the `audio`-th transcript.
    Paragraph { audio: usize, words: Range<usize> },
}

/// Word index ranges, a new one starting at each pause of `pause_ms` or more.
pub fn paragraph_ranges(words: &[Word], pause_ms: u32) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for i in 1..words.len() {
        let gap = words[i].start_ms.saturating_sub(words[i - 1].end_ms);
        if gap >= pause_ms {
            ranges.push(start..i);
            start = i;
        }
    }
    if start < words.len() {
        ranges.push(start..words.len());
    }
    ranges
}

/// Split a note body into transcript paragraphs and the text around them.
///
/// The body is the transcripts appended in order, each stored as its words
/// joined by a space, so every paragraph untouched since is found verbatim.
/// A paragraph edited by hand is not found and stays in the surrounding text:
/// only that paragraph loses its timings.
pub fn locate_paragraphs(
    content: &str,
    transcripts: &[&[Word]],
    pause_ms: u32,
) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut cursor = 0;
    for (audio, words) in transcripts.iter().enumerate() {
        for range in paragraph_ranges(words, pause_ms) {
            let text = join(&words[range.clone()]);
            let may_skip = range.len() >= MIN_WORDS_TO_SEARCH;
            let Some(at) = find_paragraph(&content[cursor..], &text, may_skip)
            else {
                continue;
            };
            let start = cursor + at;
            push_text(&mut segments, &content[cursor..start]);
            segments.push(Segment::Paragraph {
                audio,
                words: range,
            });
            cursor = start + text.len();
        }
    }
    push_text(&mut segments, &content[cursor..]);
    segments
}

fn join(words: &[Word]) -> String {
    words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_paragraph(rest: &str, text: &str, may_skip: bool) -> Option<usize> {
    if may_skip {
        return rest
            .match_indices(text)
            .map(|(at, _)| at)
            .find(|&at| on_word_bounds(rest, at, text.len()));
    }
    let lead = rest.len() - rest.trim_start().len();
    (rest[lead..].starts_with(text) && on_word_bounds(rest, lead, text.len()))
        .then_some(lead)
}

fn on_word_bounds(rest: &str, at: usize, len: usize) -> bool {
    let before = rest[..at].chars().next_back();
    let after = rest[at + len..].chars().next();
    before.is_none_or(char::is_whitespace)
        && after.is_none_or(char::is_whitespace)
}

fn push_text(segments: &mut Vec<Segment>, text: &str) {
    let text = text.trim();
    if !text.is_empty() {
        segments.push(Segment::Text(text.to_string()));
    }
}
