// Action-intent heuristics for note-as-action. Keyword-based, locale-bound (FR + EN), zero LLM cost.
// ponytail: leading-verb heuristic; upgrade path = LLM intent classification (like reminders.rs)
// if recall on phrasing matters. A false positive is harmless: NOTE_ACTION_PROMPT replies
// "nothing to run" when the note is not an actual instruction.

// Verbs that, leading a note or chat message, signal an explicit "launch this now" intent.
const TRIGGER_VERBS: &[&str] = &[
    "lance",
    "lancer",
    "execute",
    "exécute",
    "exécuter",
    "run",
    "launch",
    "go",
];

// Broader imperative verbs signalling the text is an instruction a connected tool could perform.
const ACTION_VERBS: &[&str] = &[
    "crée",
    "créer",
    "cree",
    "creer",
    "ajoute",
    "ajouter",
    "envoie",
    "envoyer",
    "mets",
    "mettre",
    "écris",
    "écrire",
    "ecris",
    "planifie",
    "planifier",
    "réserve",
    "réserver",
    "reserve",
    "supprime",
    "supprimer",
    "create",
    "add",
    "send",
    "put",
    "write",
    "schedule",
    "book",
    "delete",
    "remove",
    "make",
    "set",
    "update",
];

// First alphabetic run of the text, lowercased. Skips leading bullets/quotes/punctuation so
// "- Crée ...", "« lance ...", "lance-le" all resolve to their leading verb.
fn leading_word(text: &str) -> String {
    text.trim_start_matches(|c: char| !c.is_alphabetic())
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase()
}

/// Narrow: the text opens with an explicit launch verb ("lance xxx", "run xxx").
/// Used to route a chat message to the action path; strict to avoid hijacking real questions.
pub fn is_action_trigger(text: &str) -> bool {
    TRIGGER_VERBS.contains(&leading_word(text).as_str())
}

/// Broad: the text opens with an imperative/launch verb, i.e. it reads like an instruction.
/// Gates whether the note shows the "run this note" button.
pub fn is_actionable(text: &str) -> bool {
    let w = leading_word(text);
    TRIGGER_VERBS.contains(&w.as_str()) || ACTION_VERBS.contains(&w.as_str())
}

/// A bot message that reads like an executed-action confirmation: a single line carrying a link.
/// ponytail: presentational heuristic so the chat action card survives reload without a DB
/// migration (CHECK role IN ('user','bot') stays intact); upgrade path = persist an explicit
/// action flag if a one-line answer with a link ever gets mis-carded.
pub fn looks_like_action(text: &str) -> bool {
    let t = text.trim();
    // case-insensitive scheme: a connector could return an upper/mixed-case URL
    !t.is_empty()
        && !t.contains('\n')
        && t.to_ascii_lowercase().contains("](http")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_detects_only_explicit_launch() {
        assert!(is_action_trigger("lance la synchro du tableur"));
        assert!(is_action_trigger("Run the export"));
        assert!(is_action_trigger("exécute cette note"));
        assert!(!is_action_trigger("quelle heure est-il ?"));
        // create is an action, but not an explicit launch trigger
        assert!(!is_action_trigger("crée une feuille"));
    }

    #[test]
    fn actionable_covers_imperatives_and_trigger() {
        assert!(is_actionable("Crée une feuille de suivi"));
        assert!(is_actionable("ajoute Paul au CRM"));
        assert!(is_actionable("- envoie le compte rendu"));
        assert!(is_actionable("lance xxx"));
        // noun-led notes are not instructions
        assert!(!is_actionable("réunion lundi avec l'équipe"));
        assert!(!is_actionable("note sur le projet"));
    }

    #[test]
    fn looks_like_action_needs_single_line_with_link() {
        assert!(looks_like_action(
            "✓ Ajouté à la feuille [Suivi](https://docs.google.com/x)"
        ));
        // uppercase scheme still recognized
        assert!(looks_like_action(
            "✓ Done [Sheet](HTTPS://docs.google.com/x)"
        ));
        assert!(!looks_like_action(
            "Voici un résumé.\nPlusieurs lignes [lien](https://x)"
        ));
        assert!(!looks_like_action("Réponse simple sans lien."));
        assert!(!looks_like_action("  "));
    }
}
