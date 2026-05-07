# Instructions de Démarrage — FlowFlow

Ce fichier est le brief complet pour la première session Claude Code sur ce projet.
Copie-colle le contenu de la section "Prompt de Démarrage" dans Claude Code.

---

## Prompt de Démarrage

```
Tu travailles pour Mirko, développeur full-stack freelance basé à Bruxelles.

il faudra utiliser un max de skills a ta disposition et régulièrelment les lire, ainsi que d'utiliser exa mcp et utiliser uniquement le dernières versions de libs et tools qu'on utilisera

Ce projet est FlowFlow — une app mobile 100% Rust qui :
- Enregistre des notes vocales
- Les transcrit via Soniox REST API
- Génère des titres et tags par LLM
- Stocke tout localement (SQLite + LanceDB)
- Génère des embeddings on-device (ONNX Runtime)
- Permet de chatter avec ses notes via RAG local

Le concept vient de SuperPowerNotes (app Next.js/TypeScript existante).
L'analyse complète de la codebase source est dans ANALYSIS.md.
Les contraintes et la méthodologie sont dans CLAUDE.md.

## Stack

- Langage : 100% Rust
- UI : Dioxus (mobile natif iOS)
- DB : SQLite (rusqlite) + LanceDB (vecteurs)
- Transcription : Soniox REST API (async)
- Embeddings : ONNX Runtime (ort) on-device
- HTTP : reqwest
- Async : tokio
- Cible : iOS uniquement

## Étape 1 — Scaffold Dioxus iOS

Crée un hello world Dioxus qui compile et tourne sur le simulateur iOS.
Juste un écran avec un texte. Rien d'autre.
On valide que le tooling fonctionne avant d'aller plus loin.

Avant de commencer :
1. Lis CLAUDE.md pour comprendre les contraintes et la méthodologie
2. Lis ANALYSIS.md pour comprendre le contexte SuperPowerNotes
3. Vérifie les prérequis (rustup, cargo, Xcode, simulateur iOS)
4. Propose ton approche avant de coder

## Comment on travaille

- Une seule étape à la fois
- Tu ne prédéfinis pas la structure au-delà de l'étape en cours
- Après chaque étape → STOP → montre le résultat → attends ma validation
- Tu proposes la prochaine étape avec tes recommandations, mais je décide
- Si ça ne compile pas → corriger avant d'avancer
- Pas de suppositions — tu testes
- Git : commit après chaque étape validée
- Quand tu hésites → expose les options avec pour/contre → je tranche
```

---

## Checklist Prérequis (à vérifier avant l'étape 1)

- [ ] Rust installé (`rustup --version`)
- [ ] Target iOS ajoutée (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`)
- [ ] Xcode installé avec simulateur iOS
- [ ] Dioxus CLI installé (`cargo install dioxus-cli`)
- [ ] Git initialisé dans ce dossier

## Après l'Étape 1

Les pistes suivantes sont indicatives (voir CLAUDE.md pour la liste complète).
Mirko décidera de la priorité à chaque étape.
