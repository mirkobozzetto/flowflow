// Demo store for App Store screenshots, no personal data:
//   cargo run --example demo_store -- /tmp/flowflow-demo-en.db en   (or fr)
use flowflow::domain::{NewFolder, NewTextNote};
use flowflow::infrastructure::persistence::Database;
use std::path::PathBuf;

struct Demo {
    themes: [(&'static str, Option<usize>); 7],
    notes: [(usize, &'static str, &'static str, &'static [&'static str]); 7],
    chat_title: &'static str,
    question: &'static str,
    answer: &'static str,
}

const EN: Demo = Demo {
    themes: [
        ("Work", None), ("Product launch", Some(0)), ("Clients", Some(0)),
        ("Personal", None), ("Travel", Some(3)), ("Health", Some(3)),
        ("Ideas", None),
    ],
    notes: [
        (1, "Feedback from the beta testers",
         "Everyone loves the voice capture. Two people asked for a widget on the lock screen. Onboarding is too long: cut the third screen.",
         &["feedback", "beta"]),
        (1, "Launch plan for the October release",
         "Beta opens on the 6th for the waitlist. Press kit ready by Friday, landing page copy reviewed by Sarah. Keep the pricing announcement for the keynote, not before.",
         &["launch", "planning"]),
        (2, "Call with Nordwind Studio",
         "They want the annual plan for 12 seats. Send the proposal by Thursday, include the training session. Decision expected end of month.",
         &["sales", "follow-up"]),
        (4, "Lisbon trip, first ideas",
         "Book the flat in Alfama. Day trip to Sintra, early train. Try the pastry shop near the tram 28 stop.",
         &["travel", "lisbon"]),
        (5, "Running plan",
         "Three runs a week. Long run on Sunday, 12 km by November. Stretch after every session.",
         &["running", "habits"]),
        (6, "App idea: a shared grocery list that learns",
         "Suggest items from what we buy every week. Voice input first. Split the list by aisle.",
         &["idea", "product"]),
        (0, "Weekly review",
         "Shipped the new onboarding. Blocked on the analytics contract. Next week: hiring interviews and the launch rehearsal.",
         &["review"]),
    ],
    chat_title: "What is left before the launch?",
    question: "What is left before the launch?",
    answer: "Three things remain:\n\n1. **Press kit**, due Friday.\n2. **Onboarding**: beta testers found it too long, cut the third screen.\n3. **Pricing** stays under wraps until the keynote.\n\nThe beta opens on the 6th for the waitlist.",
};

const FR: Demo = Demo {
    themes: [
        ("Travail", None), ("Lancement produit", Some(0)), ("Clients", Some(0)),
        ("Perso", None), ("Voyages", Some(3)), ("Santé", Some(3)),
        ("Idées", None),
    ],
    notes: [
        (1, "Retours des bêta-testeurs",
         "Tout le monde adore la capture vocale. Deux personnes demandent un widget sur l'écran verrouillé. L'accueil est trop long : supprimer le troisième écran.",
         &["retours", "bêta"]),
        (1, "Plan de lancement de la version d'octobre",
         "La bêta ouvre le 6 pour la liste d'attente. Dossier de presse prêt vendredi, textes de la page d'accueil relus par Sarah. Garder l'annonce des prix pour la keynote, pas avant.",
         &["lancement", "planning"]),
        (2, "Appel avec le studio Nordwind",
         "Ils veulent l'offre annuelle pour 12 places. Envoyer la proposition d'ici jeudi, avec la formation. Décision attendue fin du mois.",
         &["vente", "relance"]),
        (4, "Lisbonne, premières idées",
         "Réserver l'appartement dans l'Alfama. Journée à Sintra, train tôt. Tester la pâtisserie près de l'arrêt du tram 28.",
         &["voyage", "lisbonne"]),
        (5, "Programme de course",
         "Trois sorties par semaine. Sortie longue le dimanche, 12 km en novembre. Étirements après chaque séance.",
         &["course", "habitudes"]),
        (6, "Idée d'app : une liste de courses partagée qui apprend",
         "Proposer ce qu'on achète chaque semaine. La voix d'abord. Trier la liste par rayon.",
         &["idée", "produit"]),
        (0, "Bilan de la semaine",
         "Nouvel accueil livré. Bloqué sur le contrat d'analytics. La semaine prochaine : entretiens d'embauche et répétition du lancement.",
         &["bilan"]),
    ],
    chat_title: "Que reste-t-il avant le lancement ?",
    question: "Que reste-t-il avant le lancement ?",
    answer: "Il reste trois choses :\n\n1. **Le dossier de presse**, prévu vendredi.\n2. **L'accueil** : les bêta-testeurs le trouvent trop long, supprimer le troisième écran.\n3. **Les prix** restent secrets jusqu'à la keynote.\n\nLa bêta ouvre le 6 pour la liste d'attente.",
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (path, lang) = (PathBuf::from(&args[1]), args[2].as_str());
    let demo = if lang == "fr" { &FR } else { &EN };
    let _ = std::fs::remove_file(&path);
    let db = Database::open_at(path).expect("open demo store");
    db.set_setting("language", lang).unwrap();
    db.set_setting("ai_consent", "true").unwrap();

    let mut theme_ids: Vec<String> = Vec::new();
    for (name, parent) in demo.themes {
        let folder = db
            .create_folder(&NewFolder {
                name: name.into(),
                description: None,
                parent_id: parent.map(|i| theme_ids[i].clone()),
            })
            .unwrap();
        theme_ids.push(folder.id);
    }
    let mut note_ids = Vec::new();
    for (theme, title, content, tags) in demo.notes.iter().rev() {
        let note = db
            .create_text_note(&NewTextNote {
                title: Some(title.to_string()),
                content: content.to_string(),
                tags: tags.iter().map(|t| t.to_string()).collect(),
            })
            .unwrap();
        db.add_note_to_folder(&note.id, &theme_ids[*theme]).unwrap();
        // Distinct creation times, so the list reads like real use.
        std::thread::sleep(std::time::Duration::from_millis(20));
        note_ids.push((note.id, *title, *content, note.created_at));
    }
    note_ids.reverse();

    let conversation = db.create_conversation(demo.chat_title).unwrap();
    db.add_message(&conversation.id, "user", demo.question, None)
        .unwrap();
    let sources: Vec<serde_json::Value> = [0usize, 1]
        .iter()
        .map(|&i| {
            let (id, title, content, created) = &note_ids[i];
            serde_json::json!({
                "note_id": id, "title": title, "chunk_text": content,
                "distance": 0.2, "created_at": created, "url": null,
            })
        })
        .collect();
    db.add_message(
        &conversation.id,
        "bot",
        demo.answer,
        Some(&serde_json::to_string(&sources).unwrap()),
    )
    .unwrap();
    println!(
        "seeded {lang}: {} themes, {} notes, 1 chat",
        theme_ids.len(),
        note_ids.len()
    );
}
