use regex::Regex;
use std::sync::OnceLock;

const HESITATION_WORDS: &[&str] = &[
    "eu+h+", "heu+m?", "hm+", "ben", "bah", "beh", "hein", "pf+", "mouais",
    "u+h+", "u+m+", "ah", "eh", "erm", "er", "like", "you know",
];

fn hesitation_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let alternation = HESITATION_WORDS.join("|");
        let pattern = format!(r"(?i)\b(?:{alternation})\b");
        Regex::new(&pattern).expect("valid hesitation regex")
    })
}

pub fn clean_hesitations(text: &str) -> String {
    let re = hesitation_regex();
    let stripped = re.replace_all(text, "").to_string();
    cleanup_punctuation(&stripped)
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
