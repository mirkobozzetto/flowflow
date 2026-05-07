# CLAUDE.md — FlowFlow

## Le Projet

**FlowFlow** — app mobile 100% Rust pour enregistrer des notes vocales, les transcrire, générer des tags/titres par IA, organiser en dossiers, et chatter avec ses notes via RAG local.

Inspiré de SuperPowerNotes (app Next.js/TypeScript de Mirko). Voir `ANALYSIS.md` pour l'analyse complète de la codebase source.

## Owner

Mirko Bozzetto — développeur full-stack freelance, Bruxelles.

## Contraintes Techniques

- **Langage** : 100% Rust. Zéro JavaScript, zéro TypeScript.
- **UI** : Dioxus (support mobile natif + Tailwind intégré)
- **Cible** : iOS uniquement (Android plus tard)
- **Base vectorielle** : LanceDB (recherche sémantique locale)
- **Métadonnées** : SQLite (rusqlite)
- **Transcription** : Soniox REST API (async, pas WebSocket)
- **Embeddings** : ONNX Runtime (ort) on-device (all-MiniLM-L6-v2)
- **HTTP** : reqwest
- **Async** : tokio

## Méthodologie de Travail

- Une seule étape à la fois. Après chaque étape → STOP → montrer le résultat → attendre validation de Mirko.
- Pas de structure de fichiers prédéfinie au-delà de l'étape en cours.
- Si ça ne compile pas ou ne fonctionne pas → corriger avant d'avancer.
- Pas de suppositions sur ce qui marchera — tester.
- Quand hésitation entre deux approches → exposer les options avec pour/contre → Mirko tranche.
- Git : commit après chaque étape validée, messages descriptifs.
- Noms de fichiers, structure, architecture évoluent au fur et à mesure. Rien n'est gravé.

## Pistes de Travail (ordre indicatif, Mirko décide)

| Piste | Description | Statut |
|-------|-------------|--------|
| A | Scaffold minimal Dioxus iOS (hello world sur simulateur) | Fait |
| B | Audio capture micro iOS (cpal + hound, sauver WAV) | Fait |
| C | Soniox REST (upload WAV → transcription) | — |
| D | Storage local (SQLite + LanceDB sur iOS) | — |
| E | Embeddings on-device (ONNX, all-MiniLM-L6-v2) | — |
| F | RAG + Chat (embed → search → contexte → LLM → réponse) | — |
| G | UI (construire au fur et à mesure des besoins) | — |

## Entités de Données (issues de SuperPowerNotes, adaptées)

### VoiceNote (entité principale)
- id (UUID), transcription, tags[], duration, fileName, createdAt, modifiedAt
- Nouveau : embedding vector (LanceDB), summary (LLM)

### Folder (hiérarchie)
- id, name, description, parentId (self-ref), createdAt
- Relation N:N avec VoiceNote via junction table

### User (local-first)
- id, name, timeLimit, currentPeriodRemainingTime, currentPeriodUsedTime, lastResetDate
- Pas d'OAuth — auth biometric/PIN iOS

## Pipeline Principal

```
Capture micro → WAV/audio
    → Soniox REST API → transcription
    → LLM API → titre + tags
    → ONNX (ort) → embedding vector
    → SQLite (métadonnées) + LanceDB (vecteur)
```

## Stack Versions

- Dioxus 0.7 (CLI dx 0.7.7)
- Rust 1.94.1
- iOS targets : aarch64-apple-ios, aarch64-apple-ios-sim

## Commandes

```bash
# Build
cargo build --features mobile

# Dev — simulateur iOS (dans un terminal séparé)
dx serve --ios

# Dev — device physique (iPhone branché USB ou pairé Wi-Fi)
dx serve --ios --device "iPhone de Mirko"

# Simulateur
open /Applications/Xcode.app/Contents/Developer/Applications/Simulator.app
xcrun simctl boot "iPhone 17 Pro"
xcrun simctl shutdown all

# Device management
xcrun devicectl list devices
xcrun devicectl manage pair --device <DEVICE_ID>
```

## Setup Device Physique (une seule fois)

1. iPhone : Settings → Privacy & Security → Developer Mode → activer → redémarrer
2. Brancher en USB, accepter "Trust This Computer"
3. `xcrun devicectl manage pair --device <ID>` (résout le "no DDI")
4. Xcode → Settings → Apple Accounts → clic sur compte → Manage Certificates → + → Apple Development
5. Si certificat pas reconnu par codesigning : installer le certificat intermédiaire Apple WWDR :
   `curl -sO https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer && security add-certificates AppleWWDRCAG3.cer && rm AppleWWDRCAG3.cer`
6. Vérifier : `security find-identity -v -p codesigning` doit montrer "Apple Development"
7. Créer le provisioning profile (obligatoire, compte gratuit) :
   WORKAROUND : cette méthode est lourde. On cherche un moyen plus simple.
   En attendant, cette étape crée un projet Swift/SwiftUI TEMPORAIRE dans Xcode.
   C'est la seule façon de générer un provisioning profile avec un compte Apple gratuit.
   Le profil est lié au bundle ID (com.mirkobozzetto.flowflow), pas au langage.
   Une fois créé, on supprime le projet Xcode — le profil reste et `dx serve` l'utilise pour notre app Rust.
   - Xcode → File → New → Project → iOS → App
   - Product Name : `flowflow`, Organization Identifier : `com.mirkobozzetto`, Team : Personal Team
   - Interface : SwiftUI, Language : Swift (on s'en fiche, c'est temporaire)
   - Sauver dans /tmp
   - Sélectionner iPhone comme destination en haut de Xcode
   - Cmd+R pour build — Xcode crée le provisioning profile automatiquement
   - Accepter le profil dev sur iPhone : Settings → General → VPN & Device Management → Trust
   - Fermer le projet Xcode (le profile reste dans ~/Library/Developer/Xcode/UserData/Provisioning Profiles/)
8. Après premier pairing, Wi-Fi fonctionne (même réseau)

## Références

- `ANALYSIS.md` : analyse complète de SuperPowerNotes
- `INSTRUCTIONS.md` : brief de démarrage pour la première session
- SuperPowerNotes source : `/Users/mirkobozzetto/stuffs/superpowernotes`
