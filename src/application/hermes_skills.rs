//! Finding a Hermes skill among dozens: the most used first, the rest by
//! category, a "/" query, and the skills a typed word names.

use crate::infrastructure::hermes::Skill;

/// How many skills a shortlist shows: most used, "/" matches.
pub const SHORTLIST: usize = 5;
/// How many skills the typed words may suggest at once.
pub const SUGGESTED: usize = 3;
// The word being typed suggests from this many letters on.
const PREFIX_FROM: usize = 3;
// A name part shorter than this never stands for its skill alone.
const PART_FROM: usize = 4;

// Parts of a skill name too common to mean the skill on their own.
const COMMON: &[&str] = &[
    "agent", "agents", "action", "actions", "items", "code", "review", "plan",
    "content", "video", "music", "design", "search", "monitor", "pipeline",
    "writing", "paper", "debugging", "development", "service", "apps", "web",
    "meeting", "notes", "skill", "authoring", "workflow", "workflows",
    "routing", "model", "documents", "document", "recovery", "page", "news",
    "management", "repo", "issues", "issue", "system", "reading", "feeds",
    "driven", "test", "inspection", "inspect", "serving", "harness",
    "proposals",
];

/// Lowercase, accents dropped: "Hermès" reads "hermes".
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

fn by_use(list: &mut [Skill]) {
    list.sort_by(|a, b| b.usage.cmp(&a.usage));
}

pub fn most_used(skills: &[Skill]) -> Vec<Skill> {
    let mut list = skills.to_vec();
    by_use(&mut list);
    list.truncate(SHORTLIST);
    list
}

/// Categories by name, each with its skills, most used first.
pub fn by_category(skills: &[Skill]) -> Vec<(String, Vec<Skill>)> {
    let mut groups: Vec<(String, Vec<Skill>)> = Vec::new();
    for s in skills {
        match groups.iter_mut().find(|(c, _)| *c == s.category) {
            Some((_, list)) => list.push(s.clone()),
            None => groups.push((s.category.clone(), vec![s.clone()])),
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, list) in groups.iter_mut() {
        by_use(list);
    }
    groups
}

/// "software-development" reads "Software development".
pub fn category_label(category: &str) -> String {
    let text = category.replace('-', " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The SF Symbol drawn for a category (one per category, none per skill).
pub fn category_symbol(category: &str) -> &'static str {
    match category {
        "autonomous-ai-agents" => "cpu",
        "business-development" => "briefcase",
        "creative" => "paintpalette",
        "devops" => "server.rack",
        "email" => "envelope",
        "github" => "chevron.left.forwardslash.chevron.right",
        "media" => "play.rectangle",
        "mlops" => "brain",
        "note-taking" => "note.text",
        "personalization" => "person.crop.circle",
        "productivity" => "checklist",
        "research" => "magnifyingglass",
        "smart-home" => "house",
        "social-media" => "bubble.left.and.bubble.right",
        "software-development" => "hammer",
        "web" => "globe",
        _ => "sparkles",
    }
}

/// The "/frag" ending the text (at its start or after a space).
pub fn slash_query(text: &str) -> Option<String> {
    let at = text.rfind('/')?;
    let before_ok = text[..at].chars().next_back().is_none_or(char::is_whitespace);
    let frag = &text[at + 1..];
    let word = frag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    (before_ok && word).then(|| fold(frag))
}

/// The text without its trailing "/frag".
pub fn strip_slash(text: &str) -> String {
    match (slash_query(text), text.rfind('/')) {
        (Some(_), Some(at)) => text[..at].to_string(),
        _ => text.to_string(),
    }
}

/// The best "/" matches: names starting with the query first, then use.
pub fn slash_matches(skills: &[Skill], query: &str) -> Vec<Skill> {
    let mut hits: Vec<Skill> = skills
        .iter()
        .filter(|s| fold(&s.name).contains(query) || fold(&s.category).contains(query))
        .cloned()
        .collect();
    hits.sort_by(|a, b| {
        let starts = |s: &Skill| fold(&s.name).starts_with(query);
        starts(b).cmp(&starts(a)).then(b.usage.cmp(&a.usage))
    });
    hits.truncate(SHORTLIST);
    hits
}

fn name_parts(name: &str) -> Vec<String> {
    let name = fold(name);
    let mut parts: Vec<String> = name
        .split('-')
        .filter(|p| p.chars().count() >= PART_FROM && !COMMON.contains(p))
        .map(String::from)
        .collect();
    parts.push(name);
    parts
}

/// Skills the text names: a finished word equal to a name or a distinctive
/// part of it, or the word being typed as a prefix. Never one already taken.
pub fn named(skills: &[Skill], text: &str, taken: &[String]) -> Vec<Skill> {
    let folded = fold(text);
    let mut words: Vec<&str> = folded.split(|c: char| !c.is_alphanumeric()).collect();
    let typing = if folded.ends_with(char::is_alphanumeric) {
        words.pop().unwrap_or_default()
    } else {
        ""
    };
    let done: Vec<&str> = words.into_iter().filter(|w| w.chars().count() >= 3).collect();
    let mut hits: Vec<Skill> = skills
        .iter()
        .filter(|s| !taken.contains(&s.name))
        .filter(|s| {
            let parts = name_parts(&s.name);
            parts.iter().any(|p| done.contains(&p.as_str()))
                || (typing.chars().count() >= PREFIX_FROM
                    && parts.iter().any(|p| p.starts_with(typing)))
        })
        .cloned()
        .collect();
    by_use(&mut hits);
    hits.truncate(SUGGESTED);
    hits
}
