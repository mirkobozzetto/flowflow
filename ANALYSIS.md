# Analyse SuperPowerNotes — Référence pour FlowFlow

Analyse complète de la codebase SuperPowerNotes (`/Users/mirkobozzetto/stuffs/superpowernotes`).
Objectif : comprendre les entités, flows et patterns à reprendre/adapter pour FlowFlow (100% Rust, mobile iOS).

---

## 1. Entités de Données

### Modèles (9 au total)

| Modèle | ID | Rôle | Pertinent FlowFlow |
|--------|----|------|---------------------|
| **User** | CUID | Utilisateur central, quota, profil | Oui — adapté local-first |
| **Account** | Composite (provider+id) | OAuth (Google) | Non — auth biometric/PIN |
| **Session** | Token unique | Sessions NextAuth | Non — JWT local |
| **VerificationToken** | Composite | Magic links email | Non |
| **Authenticator** | Composite | WebAuthn/Passkeys | Non |
| **VoiceNote** | UUID | Note vocale + transcription + tags | Oui — entité principale |
| **Folder** | CUID | Dossiers hiérarchiques (self-ref) | Oui — même concept |
| **NotesToFolders** | CUID | Junction note↔dossier (N:N) | Oui |
| **NewsletterSubscriber** | CUID | Abonnés newsletter | Non |

### Relations

```
User (1) ──→ (N) VoiceNote
User (1) ──→ (N) Folder
Folder (self) ──→ parentFolder (N:1) + subFolders (1:N)
VoiceNote (N) ←──→ (N) Folder  via NotesToFolders (junction)
```

### Champs clés VoiceNote

| Champ | Type | Usage |
|-------|------|-------|
| transcription | String | Texte transcrit (Whisper → Soniox pour FlowFlow) |
| tags | String[] | Tags générés par GPT |
| duration | Int? | Durée en secondes |
| fileName | String? | Nom du fichier audio |
| createdAt | DateTime | Date de création |

### Système de Quota Utilisateur

- `timeLimit` : 1800 secondes (30 min/mois) par défaut
- `currentPeriodRemainingTime` : temps restant dans la période
- `currentPeriodUsedTime` : temps utilisé dans la période
- `lastResetDate` : date du dernier reset mensuel
- Logique : avant transcription → vérifier quota → après transcription → décrémenter

### Conception Notable

- **Cascade delete** : suppression User → cascade sur tout (notes, dossiers, comptes)
- **Note multi-dossier** : une note peut être dans plusieurs dossiers (junction table)
- **Hiérarchie dossiers** : auto-référence via parentId, profondeur illimitée

---

## 2. Pipelines Principaux

### Pipeline A — Record → Transcribe → Tag → Store

```
[Client] Capture micro (MediaRecorder)
    ↓ chunks audio accumulés
[Client] finishRecording() → FormData(audio, duration, folderId)
    ↓ POST /api/transcribe
[Server] Auth check → Quota check (remaining >= duration ?)
    ↓
[Server] audioService.transcribeAudio(blob) → OpenAI Whisper API
    ↓ transcription text
[Server] Promise.all([
    openAIService.generateTags(transcription),
    openAIService.generateTitle(transcription)
])
    ↓ tags[] + title
[Server] audioService.saveVoiceNote()
    ├─ Prisma: create VoiceNote
    ├─ Prisma: update User quota (remaining -= duration)
    └─ Prisma: link VoiceNote ↔ Folder (junction)
    ↓
[Client] Response: {transcription, tags, fileName, duration, remainingTime, folders}
```

**Pour FlowFlow** : même pipeline mais Soniox REST au lieu de Whisper, embeddings ONNX en plus pour indexation vectorielle, stockage SQLite + LanceDB.

### Pipeline B — Gestion des Notes (CRUD)

```
fetchAllNotes()
  └─ si selectedFolderId → GET /api/folders/[id]/notes
     sinon → GET /api/voice-notes?skip=X&take=Y

saveNote(note) → PATCH /api/voice-notes/[id]
deleteNote(noteId) → DELETE /api/voice-notes/[id]

searchNotes({tags, keyword, startDate, endDate, folderId})
  └─ GET /api/notes?tags=X&keyword=Y&startDate=Z&endDate=W&folderId=Q
```

**Pour FlowFlow** : même CRUD mais SQLite local + recherche sémantique LanceDB en plus du filtre classique.

### Pipeline C — Gestion des Dossiers

```
fetchFolders() → GET /api/folders → compute rootFolders + subFolders
createFolder({name, description, parentId?}) → POST /api/folders
moveNote(noteId, targetFolderId) → POST /api/notes/[id]/move
```

Hiérarchie : rootFolders (parentId IS NULL) + subFolders groupés par parentId.
Cache : folderCacheStore pour éviter refetch.

**Pour FlowFlow** : même concept, SQLite local, drag-drop via Dioxus gestures.

### Pipeline D — Quota Utilisateur

```
1. Avant transcription : GET /api/users/[id]/usage → remainingTime
2. Check : remainingTime >= duration du recording ?
3. Après transcription : update User (usedTime += duration, remainingTime -= duration)
4. Reset mensuel automatique (lastResetDate check)
```

**Pour FlowFlow** : logique identique, stockée en SQLite local. Potentiellement pas de quota si tout est on-device (à décider).

---

## 3. Architecture en Couches (SuperPowerNotes)

```
┌─────────────────────────────────────┐
│  Components (UI React)               │
│  Recorder, NotesListing, FolderTree  │
└────────────────┬────────────────────┘
                 │
┌─────────────────────────────────────┐
│  Hooks (Orchestration)              │
│  useRecordingActions, useNoteManager │
└────────────────┬────────────────────┘
                 │
┌─────────────────────────────────────┐
│  Zustand Stores (Client State)      │
│  audioHandling, recording, noteManager, folderCache │
└────────────────┬────────────────────┘
                 │
┌─────────────────────────────────────┐
│  Services (API Client Layer)        │
│  voiceNotesService, folderService, userService │
└────────────────┬────────────────────┘
                 │
┌─────────────────────────────────────┐
│  API Routes (Next.js Backend)       │
│  /api/transcribe, /api/voice-notes, /api/folders │
└────────────────┬────────────────────┘
                 │
┌─────────────────────────────────────┐
│  Query Builders + Prisma ORM        │
│  PostgreSQL                          │
└─────────────────────────────────────┘
```

**Équivalent FlowFlow (Rust)** :
```
Dioxus Components → Dioxus Signals (state) → Services (async) → SQLite/LanceDB
                                           → reqwest (Soniox API, LLM API)
```

---

## 4. Mapping Dépendances JS → Rust

| Dep JS | Rôle | Pertinent? | Équivalent Rust |
|--------|------|------------|-----------------|
| next | Framework full-stack | Non | Dioxus (mobile) |
| react | UI web | Non | Dioxus |
| prisma | ORM PostgreSQL | Partiellement | rusqlite (SQLite) |
| next-auth | Auth OAuth | Non | JWT custom + biometric iOS |
| openai | Whisper + GPT | Non → Soniox | reqwest → Soniox REST API |
| @tanstack/react-query | Data fetching cache | Concept | reqwest + tokio |
| zustand | State management | Concept | Dioxus signals |
| zod | Validation schemas | Oui | serde + validation |
| @ffmpeg/ffmpeg | Conversion audio | Partiellement | hound + rodio |
| framer-motion | Animations | Non | Core Animation iOS |
| resend | Email | Non (mobile) | — |
| tailwindcss | CSS | Non | Dioxus/native iOS styling |
| react-dnd | Drag & drop | Concept | Dioxus gestures |

---

## 5. Ce qu'on Garde vs Ce qu'on Drop

### Garder (patterns/concepts)

- Modèle de données : User, VoiceNote, Folder, NotesToFolders (adapté SQLite)
- Pipeline record → transcribe → tag → store (adapté Soniox + ONNX)
- Système de quota (logique identique)
- Hiérarchie de dossiers avec self-référence
- Note multi-dossier (junction table)
- Recherche par tags, keyword, dates
- Validation à la désérialisation (Zod → serde)

### Drop

- Toute l'infrastructure React/Next.js
- OAuth / NextAuth / Magic Links → biometric/PIN iOS
- PostgreSQL → SQLite local
- OpenAI Whisper → Soniox REST API
- Server-side rendering
- Browser extension
- Newsletter
- Email system (pas pertinent mobile)
- Tailwind CSS → native iOS styling

### Ajouter (nouveau pour FlowFlow)

- LanceDB : base vectorielle locale pour recherche sémantique
- ONNX Runtime (ort) : embeddings on-device (all-MiniLM-L6-v2)
- RAG pipeline : question → embed → search LanceDB → contexte → LLM API → réponse
- Chat conversationnel avec les notes
- Offline-first : tout fonctionne sans connexion sauf transcription + LLM

---

## 6. Stack Rust Recommandée

```toml
[dependencies]
dioxus = "0.6"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
lancedb = "0.10"
ort = "2.0"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
hound = "3.5"
dotenvy = "0.15"
```

---

## 7. Recommandation Prochaine Étape

**Scaffold Dioxus iOS** : créer un hello world Dioxus qui compile et tourne sur simulateur iOS.
Valider que le tooling fonctionne avant d'intégrer quoi que ce soit.
C'est la Piste A — le fondement technique de tout le reste.
