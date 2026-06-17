# RFCs — FlowFlow

| ID | Title | Status | Finalized | Recommendation |
|----|-------|--------|-----------|----------------|
| 0001 | [Backup, export & restore des données FlowFlow](./0001-data-backup-export/RFC.md) | Review | — | — |
| 0002 | [Import audio dans une note pour transcription](./0002-audio-import-transcription/RFC.md) | Review | — | Implémenté |
| 0003 | [smart note-driven reminders](./0003-smart-note-driven-reminders/RFC.md) | Accepted | 2026-06-01 | Alt 4 hybride - EventKit Reminders + UN fallback (confidence medium) |
| 0004 | [Synchronisation multi-appareils (LAN, sans serveur)](./0004-multidevice-sync/RFC.md) | Accepted | 2026-06-09 | Alt 4 - sync LAN pair-à-pair: rusqlite + version vector + tombstones + Noise/PSK + BLOB vecteurs; zéro perte, rien hors des appareils; audio (fichiers) descopé v1, transcription seule (confidence medium, 2 spikes à lever) |
| 0005 | [Pluggable transcription providers with local Whisper models](./0005-pluggable-transcription-providers-with-local-whisper-models/RFC.md) | Accepted | 2026-06-11 | Alt 2 - façade enum TranscriptionClient (pattern LlmClient): Soniox inchangé + WhisperLocal (whisper-rs, spike iOS validé) + catalogue 5 modèles ggml au choix de l'utilisateur (sha256 épinglés); benchmark iPhone T01 = gate de merge (confidence medium-high) |
