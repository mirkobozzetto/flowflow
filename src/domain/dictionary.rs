use regex::Regex;
use std::collections::HashMap;

/// Similarity above which a phonetic near-match is corrected. Tuned so that a
/// legitimate French word close to a declared term is left alone: "clavier"
/// scores 0.909 against "Klavis" and must NOT be rewritten.
pub const PHONETIC_MATCH_THRESHOLD: f64 = 0.92;

/// Below this many phonetic characters almost everything resembles everything.
const MIN_PHONETIC_CHARS: usize = 4;

/// Widest run of words compared against a single declared term.
const MAX_PHONETIC_WINDOW: usize = 3;

/// A garbled term keeps roughly the length of the term and starts like it. These
/// two guards are what separate "a mangled Soniox" from "Soniox plus the words
/// around it": similarity alone rates both highly, because Jaro-Winkler rewards a
/// target fully contained in the candidate.
const MAX_PHONETIC_LEN_DRIFT: usize = 2;
const MIN_PHONETIC_SHARED_PREFIX: usize = 2;

const FIELD_SEPARATOR: char = '\t';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEntry {
    pub heard: String,
    pub correct: String,
}

impl DictionaryEntry {
    pub fn new(heard: impl Into<String>, correct: impl Into<String>) -> Self {
        Self {
            heard: heard.into(),
            correct: correct.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    entries: Vec<DictionaryEntry>,
}

impl Dictionary {
    /// One entry per line, `heard<TAB>correct`. A line with no separator declares
    /// a term whose spelling is already correct: it still feeds the engine push
    /// and the phonetic rescue.
    pub fn parse(raw: &str) -> Self {
        let mut entries = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (heard, correct) = match line.split_once(FIELD_SEPARATOR) {
                Some((h, c)) => (h.trim(), c.trim()),
                None => (line, line),
            };
            if heard.is_empty() || correct.is_empty() {
                continue;
            }
            entries.push(DictionaryEntry::new(heard, correct));
        }
        Self::from_entries(entries)
    }

    pub fn from_entries(mut entries: Vec<DictionaryEntry>) -> Self {
        // Longest first so a phrase wins over a word it contains.
        entries.sort_by(|a, b| b.heard.len().cmp(&a.heard.len()));
        Self { entries }
    }

    pub fn serialize(&self) -> String {
        self.entries
            .iter()
            .map(|e| {
                if e.heard == e.correct {
                    e.correct.clone()
                } else {
                    format!("{}{FIELD_SEPARATOR}{}", e.heard, e.correct)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[DictionaryEntry] {
        &self.entries
    }

    /// Correct spellings, deduplicated. This is what a provider with a native
    /// vocabulary field receives.
    pub fn terms(&self) -> Vec<&str> {
        let mut seen = Vec::new();
        for entry in &self.entries {
            let term = entry.correct.as_str();
            if !seen.contains(&term) {
                seen.push(term);
            }
        }
        seen
    }

    pub fn apply(&self, text: &str) -> String {
        if self.entries.is_empty() || text.is_empty() {
            return text.to_string();
        }
        let replaced = self.replace_literal(text);
        self.rescue_phonetic(&replaced)
    }

    fn replace_literal(&self, text: &str) -> String {
        let alternation = self
            .entries
            .iter()
            .map(|e| regex::escape(&e.heard))
            .collect::<Vec<_>>()
            .join("|");
        let re = match Regex::new(&format!(r"(?i)\b(?:{alternation})\b")) {
            Ok(re) => re,
            Err(e) => {
                eprintln!("[dictionary] literal pass skipped: {e}");
                return text.to_string();
            }
        };
        let lookup: HashMap<String, &str> = self
            .entries
            .iter()
            .map(|e| (e.heard.to_lowercase(), e.correct.as_str()))
            .collect();
        re.replace_all(text, |caps: &regex::Captures| {
            let matched = &caps[0];
            lookup
                .get(&matched.to_lowercase())
                .copied()
                .unwrap_or(matched)
                .to_string()
        })
        .into_owned()
    }

    fn rescue_phonetic(&self, text: &str) -> String {
        let targets: Vec<(String, &str)> = self
            .terms()
            .into_iter()
            .map(|t| (phonetic_key(t), t))
            .filter(|(key, _)| key.len() >= MIN_PHONETIC_CHARS)
            .collect();
        if targets.is_empty() {
            return text.to_string();
        }

        let words = word_spans(text);
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;
        let mut w = 0usize;

        while w < words.len() {
            // Best score across window sizes, not the widest window that passes:
            // a wider window can clear the threshold on the shared prefix alone
            // and swallow a word that belongs to the sentence.
            let mut hit: Option<(f64, usize, &str)> = None;
            let widest = MAX_PHONETIC_WINDOW.min(words.len() - w);
            for size in 1..=widest {
                let (start, _) = words[w];
                let (_, end) = words[w + size - 1];
                let candidate = phonetic_key(&text[start..end]);
                if candidate.len() < MIN_PHONETIC_CHARS {
                    continue;
                }
                if let Some((score, term)) = best_match(&candidate, &targets) {
                    if hit.map(|(best, _, _)| score > best).unwrap_or(true) {
                        hit = Some((score, size, term));
                    }
                }
            }
            match hit {
                Some((_, size, term)) => {
                    let (start, _) = words[w];
                    let (_, end) = words[w + size - 1];
                    out.push_str(&text[cursor..start]);
                    out.push_str(term);
                    cursor = end;
                    w += size;
                }
                None => w += 1,
            }
        }
        out.push_str(&text[cursor..]);
        out
    }
}

fn best_match<'a>(
    candidate: &str,
    targets: &[(String, &'a str)],
) -> Option<(f64, &'a str)> {
    let mut best: Option<(f64, &'a str)> = None;
    for (key, term) in targets {
        if !plausible_variant(candidate, key) {
            continue;
        }
        let score = if key == candidate {
            1.0
        } else {
            strsim::jaro_winkler(candidate, key)
        };
        if score >= PHONETIC_MATCH_THRESHOLD
            && best.map(|(b, _)| score > b).unwrap_or(true)
        {
            best = Some((score, term));
        }
    }
    best
}

fn plausible_variant(candidate: &str, key: &str) -> bool {
    if candidate.len().abs_diff(key.len()) > MAX_PHONETIC_LEN_DRIFT {
        return false;
    }
    let shared = candidate
        .chars()
        .zip(key.chars())
        .take_while(|(a, b)| a == b)
        .count();
    shared >= MIN_PHONETIC_SHARED_PREFIX
}

/// Byte ranges of the word tokens in `text`. An apostrophe splits a token, which
/// is what lets a two-word window rejoin "l'anceDB" into "LanceDB".
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_alphanumeric() {
            start.get_or_insert(idx);
        } else if let Some(s) = start.take() {
            spans.push((s, idx));
        }
    }
    if let Some(s) = start {
        spans.push((s, text.len()));
    }
    spans
}

/// A coarse phonetic signature shared by the French and English spellings of the
/// same sound. Not a standards-grade Double Metaphone: it only has to make a
/// mis-transcription collide with the term the user declared.
pub fn phonetic_key(text: &str) -> String {
    let folded: Vec<char> = text
        .chars()
        .flat_map(|c| c.to_lowercase())
        .flat_map(strip_accent)
        .filter(|c| c.is_alphanumeric())
        .collect();

    let mut mapped = String::with_capacity(folded.len());
    let mut i = 0;
    while i < folded.len() {
        let ch = folded[i];
        let next = folded.get(i + 1).copied();
        match ch {
            'p' if next == Some('h') => {
                mapped.push('f');
                i += 2;
            }
            'c' if next == Some('k') => {
                mapped.push('k');
                i += 2;
            }
            't' if next == Some('h') => {
                mapped.push('t');
                i += 2;
            }
            'q' => {
                mapped.push('k');
                i += if next == Some('u') { 2 } else { 1 };
            }
            'c' => {
                let soft = matches!(next, Some('e') | Some('i') | Some('y'));
                mapped.push(if soft { 's' } else { 'k' });
                i += 1;
            }
            'x' => {
                mapped.push_str("ks");
                i += 1;
            }
            'z' => {
                mapped.push('s');
                i += 1;
            }
            'y' => {
                mapped.push('i');
                i += 1;
            }
            _ => {
                mapped.push(ch);
                i += 1;
            }
        }
    }

    let mut out = String::with_capacity(mapped.len());
    let mut last = None;
    for ch in mapped.chars() {
        if last != Some(ch) {
            out.push(ch);
            last = Some(ch);
        }
    }
    out
}

fn strip_accent(c: char) -> std::vec::IntoIter<char> {
    let replacement: Vec<char> = match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => vec!['a'],
        'é' | 'è' | 'ê' | 'ë' => vec!['e'],
        'î' | 'ï' | 'í' | 'ì' => vec!['i'],
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => vec!['o'],
        'ù' | 'û' | 'ü' | 'ú' => vec!['u'],
        'ÿ' => vec!['y'],
        'ç' => vec!['c'],
        'ñ' => vec!['n'],
        'œ' => vec!['o', 'e'],
        'æ' => vec!['a', 'e'],
        other => vec![other],
    };
    replacement.into_iter()
}
