# Refacto: pub use + prelude.rs

## Objectif

Ajouter des re-exports dans chaque mod.rs + un prelude.rs global.
Equivalent Rust des "barrels" TypeScript (index.ts).
Resultat: imports courts partout, API publique claire par module.

## Etat actuel

| Module | Re-exports ? | Commentaire |
|--------|-------------|-------------|
| `models/mod.rs` | 3/4 wildcard | Manque `conversation::*` |
| `services/mod.rs` | Aucun | 10 `pub mod` sans re-export |
| `db/mod.rs` | Aucun | Database, repos, now_iso exposables |
| `ui/mod.rs` | 2 types | AppState, View seulement |
| `services/tools/mod.rs` | 3 types | Fait correctement |
| `services/transcription/mod.rs` | 2 types | Fait correctement |
| `platform/ios/mod.rs` | 5 fn | Fait correctement |
| `ui/sidebar/mod.rs` | 2 wildcard | Fait correctement |
| `ui/chat/mod.rs` | 1 type | ChatView seulement |
| `ui/notes/mod.rs` | 1 type | NoteDetail seulement |
| `ui/recording/mod.rs` | 3 types | Fait correctement |

## Top 15 imports (par frequence)

| Type | Module | Usages |
|------|--------|--------|
| `Database` | db | ~15 fichiers |
| `AppState`, `View` | ui::state | ~10 fichiers |
| `icons::*` | ui::icons | ~8 fichiers |
| `AudioRecorder`, `RecordingState` | services::audio | ~6 fichiers |
| `embed_note`, `embed_attachment` | services::embed | ~4 fichiers |
| `LlmClient` | services::llm | ~4 fichiers |
| `Note`, `NewTextNote` | models::note | ~4 fichiers |
| `Folder`, `NewFolder` | models::folder | ~3 fichiers |
| `VectorStore`, `SearchResult` | services::vectordb | ~3 fichiers |
| `SonioxClient` | services::transcription | ~2 fichiers |
| `RagSource`, `RagResponse` | services::rag | ~2 fichiers |
| `Attachment` | models::attachment | ~2 fichiers |
| `now_iso` | db | ~2 fichiers |

## Plan

### 1. pub use dans chaque mod.rs

**`src/services/mod.rs`** — ajouter:
```rust
pub use audio::{AudioRecorder, RecordingState};
pub use constants::*;
pub use embed::{embed_note, embed_attachment, delete_note_embeddings, delete_attachment_embeddings};
pub use error::LlmError;
pub use llm::{LlmClient, Provider};
pub use rag::{query as rag_query, RagResponse, RagSource};
pub use vectordb::{VectorStore, SearchResult, Chunk};
```

**`src/db/mod.rs`** — ajouter:
```rust
pub use self::database::{Database, db_path, now_iso};
// (Database struct is defined in mod.rs itself, just ensure pub)
```

**`src/models/mod.rs`** — ajouter:
```rust
pub use conversation::*;
```

**`src/ui/mod.rs`** — deja fait (AppState, View).

### 2. prelude.rs

**`src/prelude.rs`** — nouveau fichier:
```rust
pub use crate::db::Database;
pub use crate::models::{Note, NewTextNote, UpdateNote, NoteType};
pub use crate::models::{Folder, NewFolder, UpdateFolder};
pub use crate::models::{Attachment, NewAttachment};
pub use crate::services::{LlmClient, Provider, VectorStore};
pub use crate::ui::{AppState, View};
```

**`src/lib.rs`** — ajouter:
```rust
pub mod prelude;
```

### 3. Mise a jour des imports

Remplacer les chemins longs par le prelude:
```rust
// Avant
use crate::db::Database;
use crate::models::note::{Note, NewTextNote};
use crate::services::llm::LlmClient;

// Apres
use crate::prelude::*;
```

Ou par les re-exports module:
```rust
// Avant
use crate::services::vectordb::VectorStore;

// Apres
use crate::services::VectorStore;
```

## Regles

- Pas de wildcard `pub use` dans `services/mod.rs` (trop de symboles)
- Re-exports nommes explicitement
- prelude.rs = 10-15 types max (les plus frequents)
- Ne pas re-exporter les types internes (Chunk, schema, etc.)
- Tests: `cargo build --features mobile` apres chaque batch de fichiers

## Priorite

Apres Track H. Refacto pure, zero changement fonctionnel.
