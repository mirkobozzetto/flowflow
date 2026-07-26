use crate::domain::transcript::{Transcript, Word};
use regex::Regex;
use std::sync::OnceLock;

const HESITATION_WORDS: &[&str] = &[
    "eu+h+", "heu+m?", "hm+", "ben", "bah", "beh", "hein", "pf+", "mouais",
    "u+h+", "u+m+", "ah", "eh", "erm", "er", "like", "you know",
];

/// Anchored, because a word is matched whole: `\b` boundaries inside a longer
/// string would let "er" fire inside a real word once the text is split.
fn hesitation_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let alternation = HESITATION_WORDS.join("|");
        let pattern = format!(r"(?i)^(?:{alternation})$");
        Regex::new(&pattern).expect("valid hesitation regex")
    })
}

/// Widest hesitation expressed in words ("you know" is two).
fn max_hesitation_words() -> usize {
    static MAX: OnceLock<usize> = OnceLock::new();
    *MAX.get_or_init(|| {
        HESITATION_WORDS
            .iter()
            .map(|w| w.split_whitespace().count())
            .max()
            .unwrap_or(1)
    })
}

pub fn clean_hesitations(text: &str) -> String {
    let words = Transcript::from_text(text).words;
    Transcript::new(clean_hesitations_words(words)).text()
}

/// Drop the words that are pure hesitation, keeping every survivor's timing.
///
/// Matching runs against a punctuation-trimmed copy of the word, so "euh," is
/// recognised; the comma it was carrying is re-attached to the previous
/// survivor rather than vanishing with the word.
pub fn clean_hesitations_words(words: Vec<Word>) -> Vec<Word> {
    let widest_phrase = max_hesitation_words();
    let re = hesitation_regex();
    let mut out: Vec<Word> = Vec::with_capacity(words.len());
    let mut i = 0usize;

    while i < words.len() {
        let widest = widest_phrase.min(words.len() - i);
        let matched = (1..=widest).rev().find(|size| {
            let phrase = words[i..i + size]
                .iter()
                .map(|w| w.trimmed())
                .collect::<Vec<_>>()
                .join(" ");
            re.is_match(&phrase)
        });

        let Some(size) = matched else {
            out.push(words[i].clone());
            i += 1;
            continue;
        };

        let tail = words[i + size - 1].trailing_punctuation();
        if !tail.is_empty() {
            if let Some(prev) = out.last_mut() {
                prev.text =
                    cleanup_punctuation(&format!("{}{tail}", prev.text));
            }
        }
        i += size;
    }

    if let Some(first) = out.first_mut() {
        first.text = drop_leading_punct(&first.text);
    }
    out.retain(|w| !w.text.is_empty());
    out
}

fn cleanup_punctuation(text: &str) -> String {
    let collapsed = collapse_whitespace_smart(text);
    let dedup = dedup_punct(&collapsed);
    let recollapsed = collapse_whitespace_smart(&dedup);
    drop_leading_punct(&recollapsed)
}

fn collapse_whitespace_smart(text: &str) -> String {
    let punct = [',', '.', ';', ':', '!', '?'];
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !out.is_empty() {
                pending_space = true;
            }
            continue;
        }
        if punct.contains(&c) {
            pending_space = false;
            out.push(c);
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    }
    out
}

fn dedup_punct(text: &str) -> String {
    let punct = [',', '.', ';', ':', '!', '?'];
    let sentence_end = ['.', '!', '?'];
    let weak = [',', ';', ':'];
    let mut out = String::with_capacity(text.len());
    let mut last_nonspace: Option<char> = None;
    for c in text.chars() {
        if c.is_whitespace() {
            out.push(c);
            continue;
        }
        if punct.contains(&c) {
            if let Some(prev) = last_nonspace {
                if prev == c {
                    continue;
                }
                if weak.contains(&prev) && sentence_end.contains(&c) {
                    while out
                        .ends_with(|ch: char| ch.is_whitespace() || ch == prev)
                    {
                        out.pop();
                    }
                } else if (sentence_end.contains(&prev) && weak.contains(&c))
                    || (weak.contains(&prev) && weak.contains(&c))
                {
                    continue;
                }
            } else {
                continue;
            }
        }
        out.push(c);
        last_nonspace = Some(c);
    }
    out
}

fn drop_leading_punct(text: &str) -> String {
    let mut s = text.to_string();
    loop {
        let trimmed = s.trim_start();
        if trimmed.starts_with([',', ';', ':']) {
            s = trimmed[1..].to_string();
            continue;
        }
        s = trimmed.to_string();
        break;
    }
    s.trim_end().to_string()
}
