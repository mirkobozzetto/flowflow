---
feature: Smart note-driven reminders
slug: smart-note-reminders
type: tasks
source_prd: docs/prd/smart-note-reminders/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `apex` (or the implementer) to execute.

## Relevant Files

- `Cargo.toml` — ajout `objc2-event-kit` (default-features=false) + features EK*, éventuel `objc2-user-notifications` (fallback).
- `src/platform/ios/reminders.rs` — nouveau : FFI EventKit (EKEventStore, EKReminder, EKAlarm, permission off-main-thread).
- `src/platform/ios/mod.rs` — export du module reminders.
- `src/services/reminders.rs` — nouveau : logique métier (validation datetime, règle start+due, map domaine → FFI, revoke).
- `src/services/llm.rs` — tool rig `extract_reminders` (intent temporel → JSON), date courante injectée.
- `src/services/constants.rs` — prompt d'extraction d'intention.
- `src/db/schema.rs` — migration V9 : table `note_reminders` (note_id ↔ reminder_identifier).
- `src/db/note_reminder_repo.rs` — nouveau : CRUD du mapping.
- `src/ui/notes/detail.rs` — indicateur "rappel détecté", sheet de confirmation, états.
- `src/ui/state.rs` — signal(s) pour rappels détectés / en attente de confirmation.
- `src/services/i18n/locales/{fr,en}.ftl` — labels (détecté, confirmer, ignorer, permission, échec).
- `Info.plist` (pipeline make/dx) — `NSRemindersFullAccessUsageDescription` (+ legacy `NSRemindersUsageDescription`).
- `tests/` — migration V9, repo CRUD, extraction parse, validation datetime.

## Tasks

- [x] 1.0 **Gate de faisabilité iOS** _(PRD: Constraints — gate Track-F, bloquant)_ ✅ 2026-06-01
  - [x] 1.1 Ajouter `objc2-event-kit` (default-features=false, features EK* listées) au `Cargo.toml`. _(0.3.2, feature set figé §6)_
  - [x] 1.2 Écrire un appel EventKit minimal (instancier EKEventStore) derrière `#[cfg(target_os="ios")]`. _(`src/platform/ios/reminders.rs::eventkit_link_probe`)_
  - [x] 1.3 Valider `cargo build --target aarch64-apple-ios` ET `aarch64-apple-ios-sim` (link réel, deployment target 16.0). _(2026-06-01 : les 2 Finished, 0 erreur ; bug feature `NSDateComponents` corrigé → couvert par `NSCalendar`)_
  - [ ] 1.4 STOP / validation Mirko : `make all` device (signature + install + app se lance normal) → confirmer le gate complet.

- [ ] 2.0 **Permission + FFI création de rappel** _(PRD: US3, US5)_
  - [ ] 2.1 `request_reminders_access()` exécuté hors main-thread (Condvar/RcBlock), retourne accordé/refusé.
  - [ ] 2.2 Injecter `NSRemindersFullAccessUsageDescription` dans l'Info.plist final ; vérifier sa présence dans le `.app` installé.
  - [ ] 2.3 `create_reminder(title, due, start)` via EKReminder + EKAlarm + defaultCalendarForNewReminders, save → identifier ; règle start+due (éviter EKErrorNoStartDate).
  - [ ] 2.4 `remove_reminder(identifier)`.
  - [ ] 2.5 Confiner l'`EKEventStore` (`!Send`) à un thread dédié ; gérer les erreurs en `Result` propre.
  - [ ] 2.6 Validation on-device : pop-up permission visible, rappel créé apparaît dans Rappels.app et se déclenche app fermée.

- [ ] 3.0 **Extraction d'intention temporelle (LLM)** _(PRD: US1, M1, M2)_
  - [x] 3.1 `LlmClient::extract_reminders` (appel dédié, hors agent chat) : JSON `{intents:[...]}` (action, date, time, recurrence, location), absents = null. _(`src/services/llm.rs`, 7 tests verts)_
  - [x] 3.2 Date courante injectée dans le prompt ; relatives → absolu (LLM résout). _(`REMINDER_EXTRACTION_PROMPT`)_
  - [x] 3.3 Défaut 09:00 pour date sans heure, exposé via `resolved_time()` + `has_explicit_time()` (modifiable côté UI). _(`models/reminder.rs`, Q6)_
  - [ ] 3.4 Déclencheur debounce on-blur (Q2) → câblé à T09 (UI), pas ici.

- [ ] 4.0 **Persistance du mapping note ↔ rappel** _(PRD: US6, M5, M6)_
  - [x] 4.1 Migration SQLite V9 : table `note_reminders` (composants locaux, backend, intent_hash, state, `UNIQUE(note_id,intent_hash)`, CASCADE). _(7 tests verts)_
  - [x] 4.2 Repo CRUD `note_reminder_repo` (add, list, exists_by_intent_hash, set_state tombstone, delete, delete_for_note) + `NewNoteReminder::from_intent`.
  - [ ] 4.3 Anti-doublon : primitive DB faite (`UNIQUE` + `reminder_exists_by_intent_hash`) ; diff par `note_id` à la ré-édition = T07 (service).
  - [ ] 4.4 Revoke : `remove_reminder` + tombstone sur échec OS → hook `delete_note` = T08.

- [ ] 5.0 **UI détection + confirmation** _(PRD: US1, US2)_
  - [x] 5.1 Indicateur "rappel détecté" sur la note (action + date/heure résolues). _(badge plural-aware, debounce on-idle 1.2s, dismiss ; device-validé 2026-06-01)_
  - [ ] 5.2 Sheet de confirmation : titre + date + heure (+ récurrence), actions Confirmer / Ignorer.
  - [ ] 5.3 Confirmer → pipeline service (validation → FFI → mapping) ; Ignorer → rien créé.
  - [ ] 5.4 Aucune création sans confirmation explicite (jamais silencieux).
  - [ ] 5.5 Pas de date → pas d'indicateur ; date passée → avertissement.

- [ ] 6.0 **Récurrence** _(PRD: US4)_
  - [ ] 6.1 Extraction d'une récurrence RRULE-like depuis l'intention.
  - [ ] 6.2 Mapper vers `EKRecurrenceRule` (valider interval ≥ 1 pour éviter l'exception ObjC).
  - [ ] 6.3 Vérifier le redéclenchement périodique on-device.

- [ ] 7.0 **Fallback permission refusée (UserNotifications)** _(PRD: US5)_
  - [ ] 7.1 Si accès Rappels refusé, proposer un repli notification locale (UNUserNotificationCenter + trigger calendrier).
  - [ ] 7.2 Tenir compte du cap 64 pending (re-scheduling au lancement si nécessaire).
  - [ ] 7.3 La feature reste utilisable sans accès Rappels (dégradation propre).

- [ ] 8.0 **i18n + erreurs** _(PRD: US5, exceptions)_
  - [ ] 8.1 Labels FR/EN : détecté, confirmer, ignorer, texte permission, échec + retry.
  - [ ] 8.2 Messages d'erreur clairs sur échec de création (rien perdu, réessai possible).

- [ ] 9.0 **Tests** _(PRD: M1, M5, M6)_
  - [ ] 9.1 Migration V9 + repo CRUD `note_reminders`.
  - [ ] 9.2 Parsing extraction (dates relatives → absolu, défauts, null fields).
  - [ ] 9.3 Anti-doublon (intent_hash) + revoke au delete.
  - [ ] 9.4 Set de mesure M1/M2 (capture / faux positifs) sur notes échantillon.
