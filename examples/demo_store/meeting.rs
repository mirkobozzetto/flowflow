// A recorded launch meeting: timed words, chapters and a summary, as the
// app keeps them after a transcription, so the reading screens fill without
// any AI call.
use flowflow::domain::reading::Chapter;
use flowflow::domain::summary::{source_hash, Moment, Summary, SummaryItem};
use flowflow::domain::{NewTextNote, Transcript, Word};
use flowflow::infrastructure::persistence::Database;

/// Under the app's audio folder; the capture script puts an empty file there
/// so the reading bar shows its play button.
pub const AUDIO_FILE: &str = "demo-meeting.m4a";
const DURATION_SECS: u32 = 47 * 60 + 12;
const WORD_MS: u32 = 380;
const SUMMARY_MODEL: &str = "claude-haiku-5-5";

struct Meeting {
    title: &'static str,
    tags: &'static [&'static str],
    /// (start in seconds, spoken text)
    paragraphs: [(u32, &'static str); 12],
    /// (first paragraph, title, one-line summary)
    chapters: [(usize, &'static str, &'static str); 6],
    /// (paragraph said in, text) for each part of the summary.
    key_points: &'static [(usize, &'static str)],
    decisions: &'static [(usize, &'static str)],
    actions: &'static [(usize, &'static str)],
    questions: &'static [(usize, &'static str)],
}

const EN: Meeting = Meeting {
    title: "Launch kickoff meeting",
    tags: &["meeting", "launch"],
    paragraphs: [
        (4, "Thanks everyone for joining. The goal today is simple: leave this room with one plan for the October release. Beta opens on the sixth for the waitlist, so we have about three weeks. Let's go through product, marketing, pricing and support, in that order."),
        (150, "Quick status first. The new onboarding shipped on Monday and the first numbers look good. Completion went from sixty-two to seventy-eight percent. The widget on the lock screen is the request we hear most, so it stays at the top of the list."),
        (402, "Sarah, you went through the beta feedback. What stands out? Mostly good news. People love the voice capture and they use it every day. Two complaints come back again and again: the third onboarding screen is too long, and search misses notes with typos."),
        (615, "Then we cut the third screen this week. It is a small change and it removes the main drop. Search with typos is bigger: let's keep it for after the launch and say so in the release notes."),
        (848, "Marketing now. The press kit needs screenshots, the logo pack and a one-page story. Tom, can you own it? Yes, I can have a draft by Wednesday and the final version on Friday. I will need the new screenshots by Tuesday."),
        (1120, "The landing page copy is written. Sarah reviews it tomorrow. We keep one call to action, join the beta, and we remove the pricing section until the keynote."),
        (1353, "Pricing is the open topic. We said an annual plan at forty-eight euros and a team plan per seat. Nordwind Studio asked for twelve seats with training, which tells us teams are ready to pay."),
        (1630, "Decision: we announce pricing at the keynote, not before. The pricing page gets built by Friday but stays hidden. Nordwind gets their proposal on Thursday with the training session included."),
        (1874, "Support. We expect a lot of questions in the first week. Let's write ten answers for the help center before the beta opens, based on what the testers asked."),
        (2120, "On launch day, two people watch the inbox and the crash reports in the morning, and we do a short check-in at two o'clock."),
        (2388, "Let me read back the actions. Tom owns the press kit, draft Wednesday, final Friday. Sarah reviews the landing page tomorrow. I send the Nordwind proposal on Thursday. Support writes the ten help articles by next Monday."),
        (2670, "One thing we did not settle: do we open the beta to everyone on the waitlist at once, or in two waves? Let's decide that on Monday. Thanks everyone, good meeting."),
    ],
    chapters: [
        (0, "Where the launch stands", "Beta on the 6th, onboarding completion up to 78%"),
        (2, "Beta feedback", "Voice capture loved, onboarding too long, search misses typos"),
        (4, "Press kit and landing page", "Tom owns the press kit, Sarah reviews the copy"),
        (6, "Pricing", "Announced at the keynote, Nordwind proposal on Thursday"),
        (8, "Support and launch day", "Ten help answers, inbox watched on launch day"),
        (10, "Next steps", "Actions read back, beta waves decided on Monday"),
    ],
    key_points: &[
        (0, "Beta opens on the 6th for the waitlist"),
        (1, "Onboarding completion went from 62% to 78%"),
        (2, "Testers love voice capture; the third onboarding screen is too long"),
    ],
    decisions: &[
        (3, "Cut the third onboarding screen this week"),
        (3, "Search with typos waits until after the launch"),
        (7, "Announce pricing at the keynote, not before"),
    ],
    actions: &[
        (4, "Tom: press kit draft on Wednesday, final on Friday"),
        (5, "Sarah: review the landing page copy tomorrow"),
        (7, "Send the Nordwind proposal on Thursday, training included"),
        (8, "Support: ten help center answers by Monday"),
    ],
    questions: &[(11, "Open the beta to the whole waitlist at once, or in two waves?")],
};

const FR: Meeting = Meeting {
    title: "Réunion de lancement",
    tags: &["réunion", "lancement"],
    paragraphs: [
        (4, "Merci à tous d'être là. L'objectif est simple : sortir de cette réunion avec un seul plan pour la version d'octobre. La bêta ouvre le six pour la liste d'attente, il nous reste donc trois semaines. On passe le produit, le marketing, les prix et le support, dans cet ordre."),
        (150, "D'abord un point rapide. Le nouvel accueil est sorti lundi et les premiers chiffres sont bons. Le taux de complétion passe de soixante-deux à soixante-dix-huit pour cent. Le widget sur l'écran verrouillé reste la demande la plus fréquente, il garde la première place."),
        (402, "Sarah, tu as lu les retours de la bêta. Qu'est-ce qui ressort ? Surtout du bon. La capture vocale plaît beaucoup et les testeurs s'en servent tous les jours. Deux reproches reviennent sans cesse : le troisième écran d'accueil est trop long, et la recherche rate les notes avec des fautes de frappe."),
        (615, "Alors on supprime le troisième écran cette semaine. C'est un petit changement et il enlève la principale perte. La recherche tolérante aux fautes est un plus gros chantier : on la garde pour après le lancement et on le dit dans les notes de version."),
        (848, "Passons au marketing. Le dossier de presse demande des captures, le pack du logo et une page de présentation. Tom, tu t'en charges ? Oui, j'ai un brouillon mercredi et la version finale vendredi. Il me faut les nouvelles captures pour mardi."),
        (1120, "Les textes de la page d'accueil sont écrits. Sarah les relit demain. On garde un seul appel à l'action, rejoindre la bêta, et on retire la section des prix jusqu'à la keynote."),
        (1353, "Les prix restent le sujet ouvert. On avait dit une offre annuelle à quarante-huit euros et une offre équipe par place. Le studio Nordwind demande douze places avec la formation, preuve que les équipes sont prêtes à payer."),
        (1630, "Décision : on annonce les prix à la keynote, pas avant. La page des tarifs est prête vendredi mais reste cachée. Nordwind reçoit sa proposition jeudi, formation comprise."),
        (1874, "Le support. On attend beaucoup de questions la première semaine. Écrivons dix réponses pour le centre d'aide avant l'ouverture de la bêta, à partir de ce que les testeurs ont demandé."),
        (2120, "Le jour du lancement, deux personnes surveillent la boîte de réception et les rapports de plantage le matin, et on fait un court point à quatorze heures."),
        (2388, "Je relis les actions. Tom prend le dossier de presse, brouillon mercredi, version finale vendredi. Sarah relit la page d'accueil demain. J'envoie la proposition à Nordwind jeudi. Le support écrit les dix articles d'aide d'ici lundi prochain."),
        (2670, "Un point reste ouvert : on ouvre la bêta à toute la liste d'attente d'un coup, ou en deux vagues ? On tranche lundi. Merci à tous, bonne réunion."),
    ],
    chapters: [
        (0, "Où en est le lancement", "Bêta le 6, complétion de l'accueil à 78 %"),
        (2, "Retours de la bêta", "Capture vocale appréciée, accueil trop long, fautes de frappe"),
        (4, "Dossier de presse et page d'accueil", "Tom prend le dossier, Sarah relit les textes"),
        (6, "Les prix", "Annoncés à la keynote, proposition Nordwind jeudi"),
        (8, "Support et jour du lancement", "Dix réponses d'aide, boîte surveillée le jour J"),
        (10, "Prochaines étapes", "Actions relues, vagues de la bêta tranchées lundi"),
    ],
    key_points: &[
        (0, "La bêta ouvre le 6 pour la liste d'attente"),
        (1, "La complétion de l'accueil passe de 62 % à 78 %"),
        (2, "La capture vocale plaît ; le troisième écran d'accueil est trop long"),
    ],
    decisions: &[
        (3, "Supprimer le troisième écran d'accueil cette semaine"),
        (3, "La recherche tolérante aux fautes attend l'après-lancement"),
        (7, "Annoncer les prix à la keynote, pas avant"),
    ],
    actions: &[
        (4, "Tom : brouillon du dossier de presse mercredi, final vendredi"),
        (5, "Sarah : relire les textes de la page d'accueil demain"),
        (7, "Envoyer la proposition Nordwind jeudi, formation comprise"),
        (8, "Support : dix réponses du centre d'aide d'ici lundi"),
    ],
    questions: &[(11, "Ouvrir la bêta à toute la liste d'un coup, ou en deux vagues ?")],
};

/// The meeting note, in `folder_id`, newest of the store.
pub fn seed(db: &Database, lang: &str, folder_id: &str) {
    let m = if lang == "fr" { &FR } else { &EN };
    let mut words = Vec::new();
    let mut first_word = Vec::new();
    for (start, text) in m.paragraphs {
        first_word.push(words.len());
        let mut at = start * 1000;
        for token in text.split_whitespace() {
            words.push(Word::new(token, at, at + WORD_MS - 80, 0.97));
            at += WORD_MS;
        }
    }
    let transcript = Transcript::new(words);
    let content = transcript.text();

    let note = db
        .create_text_note(&NewTextNote {
            title: Some(m.title.into()),
            content: content.clone(),
            tags: m.tags.iter().map(|t| t.to_string()).collect(),
        })
        .unwrap();
    db.add_note_to_folder(&note.id, folder_id).unwrap();
    let audio = db
        .add_audio(&note.id, AUDIO_FILE, f64::from(DURATION_SECS))
        .unwrap();
    db.set_audio_transcript(&audio.id, &transcript).unwrap();

    let chapters: Vec<Chapter> = m
        .chapters
        .iter()
        .map(|(p, title, summary)| Chapter {
            start_word: first_word[*p],
            title: title.to_string(),
            summary: summary.to_string(),
        })
        .collect();
    assert!(db
        .set_audio_chapters(&audio.id, transcript.words.len(), &chapters)
        .unwrap());

    let item = |(p, text): &(usize, &str)| SummaryItem {
        text: text.to_string(),
        at: Some(Moment {
            audio_id: audio.id.clone(),
            ms: m.paragraphs[*p].0 * 1000,
        }),
    };
    let summary = Summary {
        key_points: m.key_points.iter().map(item).collect(),
        decisions: m.decisions.iter().map(item).collect(),
        actions: m.actions.iter().map(item).collect(),
        questions: m.questions.iter().map(item).collect(),
    };
    db.save_summary(
        &note.id,
        &summary,
        lang,
        SUMMARY_MODEL,
        &source_hash(&content),
    )
    .unwrap();
}
