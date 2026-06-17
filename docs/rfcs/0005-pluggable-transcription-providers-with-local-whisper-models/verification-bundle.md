---
artifact: "docs/rfcs/0005-pluggable-transcription-providers-with-local-whisper-models/RFC.md"
generated: "2026-06-11"
status: "self-checks green, device validation pending"
---

# Verification Bundle: RFC 0005 local Whisper transcription

## Already verified by ship (self-checks)

| Check | Result |
|-------|--------|
| `make format` + `make check` (fmt + clippy mobile) | clean, 0 warning |
| `cargo test --features desktop --no-default-features` | 268 passed, 11 ignored (+19 nouveaux tests) |
| Cross-compile `aarch64-apple-ios` + `aarch64-apple-ios-sim` (whisper-rs + Metal) | OK |
| Transcription réelle desktop (tiny, WAV FR 16 kHz généré via `say`) | 350 ms, transcript correct |
| sha256 du ggml-tiny téléchargé vs valeur épinglée au catalogue | identique |
| `make all` (build + sign + icon + install iPhone) | exit 0, app installée |

## Device / manual validation (MIRKO) - gate de merge

### 1. Bench T01 (gate, à poster sur #30)
1. Réglages -> Transcription -> Local (Whisper).
2. Télécharger `base` puis `small-q5_1` (dialog taille -> confirmer, barre de %).
3. Enregistrer une note vocale FR d'environ 60 s (ou garder un audio existant).
4. Section "Benchmark (dev)" en bas -> taper `base` puis `small-q5_1`.
5. Noter pour chaque modèle: durée audio, temps (ms), RSS (MB), qualité du texte.
6. Poster les chiffres sur l'issue #30. Acceptable = quelques secondes pour 60 s, pas de jetsam, pas de chauffe anormale. Q6: si le FR auto-détecté est mauvais, je rebrancherai le language hint.

### 2. Mode avion end-to-end (T10)
1. Modèle actif sélectionné, passer en mode avion.
2. Dictée (barre micro) -> texte apparaît.
3. Import audio (menu note) -> job Polling -> texte ajouté à la note.
4. Bouton Transcrire sur un audio existant -> transcription remplie.

### 3. Non-régression Soniox (T06)
1. Réglages -> Transcription -> Cloud (Soniox).
2. Dictée + import + transcrire: comportement identique à avant.

### 4. Reprise mi-job (T07)
1. Provider local, importer un audio long, force-quit pendant Polling.
2. Relancer: le job repart de zéro et se termine.

### 5. Backup (C9, T09)
1. Avec un modèle téléchargé: exporter une archive.
2. Vérifier la taille de l'archive (pas de +180 MB = pas de modèle dedans).
3. Restore sur appareil sans modèle: provider local conservé, erreur claire "Modèle Whisper non téléchargé" à la première transcription.

### 6. Garde-fous UI (T08)
1. Annuler un dialog de téléchargement -> rien ne part.
2. Supprimer le modèle actif -> la carte redevient absente, transcription suivante erreur claire.
3. Un seul téléchargement à la fois (le 2e bouton Download disparaît pendant un download).

## Reproduire le test desktop avec vrai modèle

```bash
curl -sL -o /tmp/ggml-tiny.bin "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
say -v Thomas -o /tmp/fr.aiff "Bonjour, ceci est un test." && afconvert -f WAVE -d LEI16@16000 -c 1 /tmp/fr.aiff /tmp/fr.wav
FLOWFLOW_WHISPER_MODEL=/tmp/ggml-tiny.bin FLOWFLOW_WHISPER_WAV=/tmp/fr.wav cargo test --features desktop --no-default-features --test whisper_local_test -- --nocapture
```

## Contract coverage

C1 bench harness: SHIPPED (chiffres iPhone = Mirko). C2-C7 tests verts. C8 device (étape 6). C9 test + device (étape 5). C10 device (étape 2). C11 consent vérifié dans from_db pour les deux providers.
