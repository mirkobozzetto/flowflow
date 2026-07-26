use flowflow::domain::dictionary::{phonetic_key, PHONETIC_MATCH_THRESHOLD};
use flowflow::domain::{Dictionary, DictionaryEntry, Transcript, Word};

fn terms_only(terms: &[&str]) -> Dictionary {
    Dictionary::from_entries(
        terms.iter().map(|t| DictionaryEntry::new(*t, *t)).collect(),
    )
}

/// Words 400 ms apart, so a merge is visible in the resulting span.
fn timed(texts: &[&str]) -> Vec<Word> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let start = i as u32 * 400;
            Word::new(*t, start, start + 400, 0.9)
        })
        .collect()
}

#[test]
fn parse_accepts_pairs_bare_terms_blanks_and_padding() {
    let dict = Dictionary::parse(
        "Sony Ox\tSoniox\n\n  Soniox  \n  Flow ou Flow \t FlowFlow \n\t\n",
    );
    assert_eq!(dict.entries().len(), 3);
    assert!(dict
        .entries()
        .contains(&DictionaryEntry::new("Sony Ox", "Soniox")));
    assert!(dict
        .entries()
        .contains(&DictionaryEntry::new("Soniox", "Soniox")));
    assert!(dict
        .entries()
        .contains(&DictionaryEntry::new("Flow ou Flow", "FlowFlow")));
}

#[test]
fn serialize_roundtrips_and_keeps_bare_terms_bare() {
    let raw = "Flow ou Flow\tFlowFlow\nSony Ox\tSoniox\nDioxus";
    let dict = Dictionary::parse(raw);
    let round = Dictionary::parse(&dict.serialize());
    assert_eq!(dict.entries(), round.entries());
    assert!(dict.serialize().lines().any(|l| l == "Dioxus"));
}

#[test]
fn terms_deduplicates_correct_spellings() {
    let dict = Dictionary::parse("Sony Ox\tSoniox\nsonyox\tSoniox\nDioxus");
    let mut terms = dict.terms();
    terms.sort_unstable();
    assert_eq!(terms, vec!["Dioxus", "Soniox"]);
}

#[test]
fn empty_dictionary_is_the_identity() {
    let dict = Dictionary::parse("   \n\n\t\n");
    assert!(dict.is_empty());
    let text = "Alors euh, note pour moi-meme sur Flow ou Flow.";
    assert_eq!(dict.apply(text), text);
}

#[test]
fn literal_replacement_is_case_insensitive() {
    let dict = Dictionary::parse("sony ox\tSoniox");
    assert_eq!(
        dict.apply("parce que SONY OX coute cher"),
        "parce que Soniox coute cher"
    );
    assert_eq!(
        dict.apply("parce que Sony Ox coute cher"),
        "parce que Soniox coute cher"
    );
}

#[test]
fn literal_replacement_respects_word_boundaries() {
    let dict = Dictionary::parse("flow\tFlowFlow");
    let text = "un flowchart et un workflow ne sont pas des reflows";
    assert_eq!(dict.apply(text), text);
    assert_eq!(dict.apply("le flow est casse"), "le FlowFlow est casse");
}

#[test]
fn longest_entry_wins_over_a_word_it_contains() {
    let dict = Dictionary::parse("flow\tSolo\nflow ou flow\tFlowFlow");
    assert_eq!(
        dict.apply("quand je dis Flow ou Flow ca ressort mal"),
        "quand je dis FlowFlow ca ressort mal"
    );
}

#[test]
fn phonetic_rescues_the_measured_single_word_misses() {
    let dict = terms_only(&["Soniox", "Dioxus", "Klavis"]);
    assert_eq!(
        dict.apply("parce que Sonyox coute cher"),
        "parce que Soniox coute cher"
    );
    assert_eq!(dict.apply("pareil pour Dioxu"), "pareil pour Dioxus");
    assert_eq!(
        dict.apply("il faudrait voir avec clavy"),
        "il faudrait voir avec Klavis"
    );
}

#[test]
fn phonetic_rescues_a_two_word_name_without_eating_the_sentence() {
    let dict = terms_only(&["Mirko Bozzetto"]);
    assert_eq!(
        dict.apply("quand je dis mon nom, Mirko Bose, ca donne n'importe quoi"),
        "quand je dis mon nom, Mirko Bozzetto, ca donne n'importe quoi"
    );
}

#[test]
fn phonetic_rejoins_a_term_split_by_an_apostrophe() {
    let dict = terms_only(&["LanceDB"]);
    assert_eq!(
        dict.apply("pareil pour l'anceDB, la base vectorielle"),
        "pareil pour LanceDB, la base vectorielle"
    );
}

#[test]
fn phonetic_never_touches_a_legitimate_word_close_to_a_term() {
    let dict = terms_only(&["Klavis", "Soniox", "LanceDB", "Dioxus"]);
    // "clavier" scores 0.909 against "Klavis": below the threshold on purpose.
    // The guardrail outranks the catch: never break a correct word.
    let text = "j'ai pose le clavier sur la table et la sonde etait la";
    assert_eq!(dict.apply(text), text);
}

#[test]
fn phonetic_leaves_unrelated_text_alone() {
    let dict = terms_only(&["Soniox", "Mirko Bozzetto"]);
    let text = "Troisieme truc, il faudrait voir la semaine prochaine.";
    assert_eq!(dict.apply(text), text);
}

#[test]
fn short_tokens_are_never_phonetically_rescued() {
    let dict = terms_only(&["Soniox"]);
    let text = "son et sol ne sont pas des termes";
    assert_eq!(dict.apply(text), text);
}

#[test]
fn phonetic_key_collapses_french_and_english_spellings_of_one_sound() {
    assert_eq!(phonetic_key("Soniox"), phonetic_key("Sonyox"));
    assert_eq!(phonetic_key("Klavis"), phonetic_key("klavis"));
    assert_eq!(phonetic_key("Bozzetto"), "boseto");
    assert_eq!(phonetic_key("LanceDB"), "lansedb");
    assert_eq!(phonetic_key("Dioxus"), "dioksus");
    assert_eq!(phonetic_key("l'anceDB"), "lansedb");
    assert_eq!(phonetic_key("Ecole"), phonetic_key("École"));
    assert_eq!(phonetic_key("photo"), "foto");
    assert_eq!(phonetic_key("quai"), "kai");
}

#[test]
fn threshold_is_a_named_constant_in_the_documented_band() {
    assert!(PHONETIC_MATCH_THRESHOLD > 0.85);
    assert!(PHONETIC_MATCH_THRESHOLD < 1.0);
}

#[test]
fn word_level_multi_word_merge_spans_first_start_to_last_end() {
    let dict = Dictionary::parse("Sony Ox\tSoniox");
    let out = dict.apply_words(timed(&["parce", "que", "Sony", "Ox", "coute"]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["parce", "que", "Soniox", "coute"]
    );
    let merged = &out[2];
    assert_eq!((merged.start_ms, merged.end_ms), (800, 1600));
}

#[test]
fn word_level_apostrophe_rescue_stays_inside_one_word_and_keeps_its_timing() {
    let dict = terms_only(&["LanceDB"]);
    let out =
        dict.apply_words(timed(&["pareil", "pour", "l'anceDB,", "la", "base"]));
    assert_eq!(
        out.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["pareil", "pour", "LanceDB,", "la", "base"]
    );
    assert_eq!((out[2].start_ms, out[2].end_ms), (800, 1200));
}

#[test]
fn word_level_correction_preserves_the_stored_text_invariant() {
    let dict = Dictionary::parse("Sony Ox\tSoniox\nl'anceDB\tLanceDB");
    let out = dict.apply_words(timed(&[
        "avec",
        "Sony",
        "Ox",
        "et",
        "l'anceDB,",
        "voilà",
    ]));
    let transcript = Transcript::new(out);
    assert_eq!(
        transcript.text().split_whitespace().count(),
        transcript.words.len()
    );
    assert_eq!(transcript.text(), "avec Soniox et LanceDB, voilà");
}

#[test]
fn word_level_apply_is_the_identity_without_entries() {
    let dict = Dictionary::parse("");
    let words = timed(&["rien", "à", "corriger"]);
    assert_eq!(dict.apply_words(words.clone()), words);
}
