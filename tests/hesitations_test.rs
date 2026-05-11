use flowflow::services::transcription::clean_hesitations;

#[test]
fn test_empty_input() {
    assert_eq!(clean_hesitations(""), "");
}

#[test]
fn test_no_hesitations() {
    let text = "Bonjour, je suis un développeur freelance.";
    assert_eq!(clean_hesitations(text), text);
}

#[test]
fn test_french_isolated_euh() {
    let input = "Je voulais euh dire que c'est important.";
    assert_eq!(
        clean_hesitations(input),
        "Je voulais dire que c'est important."
    );
}

#[test]
fn test_french_with_extra_letters() {
    let input = "Alors euuuuh on commence.";
    assert_eq!(clean_hesitations(input), "Alors on commence.");
}

#[test]
fn test_hmm_at_start() {
    let input = "Hmm, je pense que oui.";
    assert_eq!(clean_hesitations(input), "je pense que oui.");
}

#[test]
fn test_english_um() {
    let input = "I think um we should go.";
    assert_eq!(clean_hesitations(input), "I think we should go.");
}

#[test]
fn test_english_you_know() {
    let input = "It's hard, you know, to explain.";
    assert_eq!(clean_hesitations(input), "It's hard, to explain.");
}

#[test]
fn test_multiple_hesitations() {
    let input = "Euh, ben, je sais pas hein.";
    assert_eq!(clean_hesitations(input), "je sais pas.");
}

#[test]
fn test_preserve_real_words() {
    let input = "Eric et Aurélie travaillent ensemble.";
    assert_eq!(clean_hesitations(input), input);
}

#[test]
fn test_pfff() {
    let input = "Pfff, c'est compliqué.";
    assert_eq!(clean_hesitations(input), "c'est compliqué.");
}

#[test]
fn test_mouais() {
    let input = "Mouais, on verra bien.";
    assert_eq!(clean_hesitations(input), "on verra bien.");
}

#[test]
fn test_collapse_whitespace() {
    let input = "Mot   euh    final.";
    assert_eq!(clean_hesitations(input), "Mot final.");
}
