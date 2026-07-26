use serde::{Deserialize, Serialize};

/// Characters that glue to a word rather than standing on their own. A word is a
/// timing unit, so punctuation never gets a `Word` of its own: it has no
/// independent time range.
const PUNCTUATION: &[char] = &[
    ',', '.', ';', ':', '!', '?', '"', '\'', '(', ')', '[', ']', '{', '}', '«',
    '»', '-', '\u{2026}',
];

/// One spoken word with the time range it occupies in the audio.
///
/// Field names are abbreviated on the wire because the array is stored as JSON
/// once per word: a one-hour note holds roughly nine thousand of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    #[serde(rename = "t")]
    pub text: String,
    #[serde(rename = "s")]
    pub start_ms: u32,
    #[serde(rename = "e")]
    pub end_ms: u32,
    #[serde(rename = "c")]
    pub confidence: f32,
}

impl Word {
    pub fn new(
        text: impl Into<String>,
        start_ms: u32,
        end_ms: u32,
        confidence: f32,
    ) -> Self {
        Self {
            text: text.into(),
            start_ms,
            end_ms,
            confidence,
        }
    }

    /// A word with no timing, produced when a transcript is rebuilt from plain
    /// text (dictation, or a transcription stored before this feature existed).
    pub fn untimed(text: impl Into<String>) -> Self {
        Self::new(text, 0, 0, 0.0)
    }

    /// The word without its surrounding punctuation. Matching - hesitation
    /// regex, dictionary lookup - runs against this, never against `text`, so
    /// "euh," is recognised as a hesitation and "flow." as the term "flow".
    pub fn trimmed(&self) -> &str {
        self.text.trim_matches(|c| PUNCTUATION.contains(&c))
    }

    /// The punctuation hanging off the end of the word. When a hesitation is
    /// dropped this is what has to survive it, re-attached to the word before.
    pub fn trailing_punctuation(&self) -> &str {
        let kept = self
            .text
            .trim_end_matches(|c| PUNCTUATION.contains(&c))
            .len();
        &self.text[kept..]
    }
}

/// An ordered list of words. The stored transcript text is derived from it, which
/// is what keeps the rendered spans and the stored string in lockstep.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    pub words: Vec<Word>,
}

impl Transcript {
    pub fn new(words: Vec<Word>) -> Self {
        Self { words }
    }

    /// Rebuild a transcript from plain text, with no timings. Punctuation that
    /// stands alone is glued to the word before it, so the "no punctuation-only
    /// word" rule holds for text of any origin.
    pub fn from_text(text: &str) -> Self {
        let mut words: Vec<Word> = Vec::new();
        for token in text.split_whitespace() {
            let is_punct_only = token.chars().all(|c| PUNCTUATION.contains(&c));
            match words.last_mut() {
                Some(prev) if is_punct_only => prev.text.push_str(token),
                _ => words.push(Word::untimed(token)),
            }
        }
        Self { words }
    }

    /// The canonical stored string. A plain join is what makes the invariant
    /// structural: every word contributes exactly one whitespace-separated token,
    /// so the span count can never drift from the word count.
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Index of the word being spoken at `ms`, for the playback highlight. Falls
    /// on the last word that started at or before `ms`, so the highlight holds
    /// through the silence between two words instead of flickering off.
    pub fn word_index_at_ms(&self, ms: u32) -> Option<usize> {
        let last = self.words.iter().rposition(|w| w.start_ms <= ms)?;
        if ms > self.words[last].end_ms && last + 1 == self.words.len() {
            return None;
        }
        Some(last)
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether the words carry real timings. A transcript rebuilt from text has
    /// none, and storing it as a word array would give the UI taps that all seek
    /// to zero.
    pub fn has_timings(&self) -> bool {
        self.words.iter().any(|w| w.end_ms > 0)
    }
}

impl From<Vec<Word>> for Transcript {
    fn from(words: Vec<Word>) -> Self {
        Self { words }
    }
}

/// A rewrite of a byte range of the joined word text.
///
/// This is how a matcher that works on a string - the dictionary windows across
/// word boundaries and inside a word alike - reports what it changed without
/// losing which words the change covered. The knowledge is kept at match time,
/// never reconstructed afterwards by diffing.
#[derive(Debug, Clone, PartialEq)]
pub struct Rewrite {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Apply non-overlapping, ascending byte-range rewrites over `words.join(" ")`
/// and map each one back onto the words it covered.
///
/// A rewrite inside a single word keeps that word's timing. One spanning several
/// words collapses them into a single word running from the first start to the
/// last end. A replacement that itself contains a space yields several words
/// sharing the collapsed span, so no word ever carries whitespace.
pub fn apply_rewrites(words: Vec<Word>, rewrites: &[Rewrite]) -> Vec<Word> {
    if rewrites.is_empty() {
        return words;
    }
    let joined = Transcript::new(words.clone()).text();
    let ranges = word_ranges(&words);

    let mut out: Vec<Word> = Vec::with_capacity(words.len());
    let mut wi = 0usize;
    let mut ri = 0usize;

    while wi < words.len() {
        while ri < rewrites.len() && rewrites[ri].end <= ranges[wi].0 {
            ri += 1;
        }
        if ri >= rewrites.len() || rewrites[ri].start >= ranges[wi].1 {
            out.push(words[wi].clone());
            wi += 1;
            continue;
        }

        let (group_end, last_rewrite) = grow_group(&ranges, rewrites, wi, ri);
        let base = (ranges[wi].0, ranges[group_end].1);
        let spliced = splice(&joined, base, &rewrites[ri..=last_rewrite]);
        out.extend(rebuild(&words[wi..=group_end], &spliced));

        wi = group_end + 1;
        ri = last_rewrite + 1;
    }
    out
}

fn word_ranges(words: &[Word]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::with_capacity(words.len());
    let mut offset = 0usize;
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            offset += 1; // the separator introduced by the join
        }
        ranges.push((offset, offset + word.text.len()));
        offset += word.text.len();
    }
    ranges
}

/// Widen a word group until it covers its rewrite, then keep widening while the
/// next rewrite still starts inside it.
fn grow_group(
    ranges: &[(usize, usize)],
    rewrites: &[Rewrite],
    first_word: usize,
    first_rewrite: usize,
) -> (usize, usize) {
    let mut group_end = first_word;
    let mut last = first_rewrite;
    loop {
        while group_end + 1 < ranges.len()
            && rewrites[last].end > ranges[group_end + 1].0
        {
            group_end += 1;
        }
        if last + 1 < rewrites.len()
            && rewrites[last + 1].start < ranges[group_end].1
        {
            last += 1;
        } else {
            return (group_end, last);
        }
    }
}

fn splice(joined: &str, base: (usize, usize), rewrites: &[Rewrite]) -> String {
    let (base_start, base_end) = base;
    let mut out = String::new();
    let mut cursor = base_start;
    for rewrite in rewrites {
        let start = rewrite.start.clamp(cursor, base_end);
        let end = rewrite.end.clamp(start, base_end);
        out.push_str(&joined[cursor..start]);
        out.push_str(&rewrite.text);
        cursor = end;
    }
    out.push_str(&joined[cursor..base_end]);
    out
}

/// Redistribute a rewritten group's text over the group's own time span.
fn rebuild(group: &[Word], spliced: &str) -> Vec<Word> {
    let start = group[0].start_ms;
    let end = group[group.len() - 1].end_ms;
    let confidence =
        group.iter().map(|w| w.confidence).sum::<f32>() / group.len() as f32;
    words_from_span(spliced, start, end, confidence)
}

/// Split a timed stretch of text into whitespace-free words sharing that stretch.
///
/// Used wherever one timed unit turns out to hold several words: a whisper.cpp
/// segment that did not split cleanly, or a dictionary replacement that expands
/// into two. Splitting the span evenly is a placeholder for timings nobody
/// measured, and it keeps the no-whitespace-in-a-word rule true.
pub fn words_from_span(
    text: &str,
    start_ms: u32,
    end_ms: u32,
    confidence: f32,
) -> Vec<Word> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let count = tokens.len() as u32;
    if count == 0 {
        return Vec::new();
    }
    let span = end_ms.saturating_sub(start_ms);
    tokens
        .iter()
        .enumerate()
        .map(|(i, token)| {
            let i = i as u32;
            Word::new(
                *token,
                start_ms + span * i / count,
                start_ms + span * (i + 1) / count,
                confidence,
            )
        })
        .collect()
}
