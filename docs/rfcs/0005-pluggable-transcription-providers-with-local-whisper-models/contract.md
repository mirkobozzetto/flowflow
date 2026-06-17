---
artifact: "docs/rfcs/0005-pluggable-transcription-providers-with-local-whisper-models/RFC.md"
artifact_kind: "rfc"
locked: "2026-06-11"
---

# Definition of Done: Pluggable transcription providers with local Whisper models

> Immutable target. Every item below is a concrete, checkable condition the final verification bundle validates against. Requirement changes get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | Bench harness transcrit une note FR de 60 s avec au moins base et small-q5_1, rapporte durée + RSS pic; Mirko poste les chiffres iPhone sur #30; PAS DE MERGE avant chiffres acceptables (Q2/Q6 répondues) | RFC T01 | device bench (USER) - MERGE GATE |
| C2 | Catalogue 5 modèles (tiny, base, small-q5_1, medium-q5_0, large-v3-turbo-q5_0): list/download(.part + sha256 + rename)/delete/active sur tempdir; garde-fou disque (2x taille) testé; vérif sha256 + taille post-download | RFC T02 | `cargo test` model manager |
| C3 | WAV 44.1k stéréo -> mono 16k -> transcript sur test desktop avec fixture modèle tiny; Semaphore(1) appliqué | RFC T03 | `cargo test` whisper backend |
| C4 | `TranscriptionClient::from_db` défaute sur Soniox quand les clés settings sont absentes; WhisperLocal erreur claire si modèle manquant; tests unitaires de dispatch | RFC T04 | `cargo test` dispatch |
| C5 | Migration `pending_transcriptions.provider` DEFAULT 'soniox': anciennes rows lues avec défaut; nouvelle row locale fait l'aller-retour | RFC T05 | `cargo test` migration |
| C6 | Dictée + transcription manuelle compilent et marchent avec provider=soniox EXACTEMENT comme avant (zéro diff de comportement) | RFC T06 | tests existants verts + device (USER) |
| C7 | Job local: enqueue -> ticks Polling -> Done -> cleanup; relance mi-job re-exécute; pipeline Soniox intouché (tests existants verts) | RFC T07 | `cargo test` job manager |
| C8 | Picker persiste `stt_provider`; cartes modèles affichent absent/téléchargement %/téléchargé; download (dialog taille, Q4)/delete/set-active marchent sur device | RFC T08 | device test (USER) |
| C9 | Archive export ne contient aucun fichier modèle; restore sur appareil sans modèles garde le réglage provider mais retombe avec erreur claire | RFC T09 | `cargo test` backup + device (USER) |
| C10 | Mode avion: dictée + import + transcription manuelle produisent du texte avec modèle local; chemin Soniox re-validé; 249+ tests verts; make all | RFC T10 | device E2E (USER) |
| C11 | Consentement IA global appliqué à TOUS les providers, local inclus (pas de bypass) | RFC Q3 decided | Read-back + device (USER) |

## Out of scope (never build)

- PAS d'AppleSpeech (SpeechAnalyzer iOS 26+) dans cette implémentation; juste un slot dans l'enum.
- PAS de transcription streaming live (mot à mot pendant l'enregistrement); record-then-transcribe sur le WAV fini.
- PAS de providers cloud au-delà de Soniox (pas de catalogue Groq/Deepgram/ElevenLabs).
- PAS de sync des modèles téléchargés entre appareils (artefacts par appareil, exclus de l'export backup).
- PAS de changement de `clean_hesitations` ni du schéma de stockage des transcripts.
- PAS de détection du type de réseau (Q4 decided: dialog taille + confirmation, c'est tout).

## Edit scope

- `src/services/transcription/provider.rs` (new), `whisper.rs` (new), `models.rs` (new), `mod.rs`
- `src/services/transcription/client.rs`: INTOUCHABLE (byte-for-byte)
- `src/services/constants.rs`
- `src/db/settings_repo.rs`, `src/db/schema.rs`
- `src/ui/transcription_manager.rs`, `src/ui/recording/controls.rs`, `src/ui/notes/audio_section.rs`
- `src/ui/settings/transcription.rs`
- `src/services/backup.rs`
- `src/services/i18n/locales/{en,fr}.ftl`
- `Cargo.toml`, `CLAUDE.md`, `README.md`
- `tests/` (model manager, whisper, dispatch, migration, job manager)
