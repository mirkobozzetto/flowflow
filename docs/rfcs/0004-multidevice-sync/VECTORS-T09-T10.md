# Passe 3 - Vecteurs (T09 id déterministe, T10 BLOB + fix store_chunks)

> But: rendre les vecteurs synchronisables (variante B du RFC). Le vecteur voyage en BLOB avec la
> note; le 2e appareil reconstruira LanceDB depuis le BLOB sans re-payer l'embedding (T12). SQLite
> devient la source de vérité des vecteurs, LanceDB un cache reconstructible.

## Ce qui a été fait

### T09 - id de chunk note déterministe
- `embed.rs`: id note `note:{note_id}:{idx}` (était `Uuid::new_v4()` aléatoire). Attachment
  `att:{attachment_id}:{idx}` inchangé. Un id stable = clé de diff entre appareils.

### T10 - BLOB SQLite + scope par préfixe (corrige BLOCKER 5)
- **`vectordb.rs`**: `owner_prefix(id)` -> `note:{owner}:` / `att:{owner}:`. `store_chunks` supprime
  désormais `id LIKE 'prefix%'` au lieu de `note_id = X`. **Signature et schéma Arrow inchangés**
  (symbole CRITICAL, 11 appelants / 7 flux RAG). Corrige le bug live: embarquer un attachment
  (qui porte `note_id` = note parente) effaçait les chunks de la note.
- **`db/chunk_repo.rs`** (nouveau): `ChunkRecord`, `replace_chunks` (DELETE owner + INSERT en UNE
  transaction = re-embed atomique, 0 orphelin), `vector_to_blob`/`blob_to_vector` (f32 little-endian,
  6144 octets), `count_chunks_for_owner`, `chunks_for_owner` (lecture pour T12), `delete_chunks_for_owner`.
- **`embed.rs`**: `persist_chunk_blobs` écrit chaque vecteur en BLOB + un `content_hash` sha256 dans
  la table `chunks` SQLite, en plus de LanceDB. Pre-delete nuisibles retirés (le `delete_note_chunks`
  avant ré-embed effaçait aussi les attachments).

### Fixes issus de la revue adversariale (avant commit)
La revue ultracode (4 lentilles: scope LanceDB, BLOB SQLite, zéro-perte, contrat) a levé 2 BLOCKER +
2 MAJOR. Corrigés:
- **Suppression -> orphelins (BLOCKER):** supprimer une note/attachment ne nettoyait pas la table
  `chunks` SQLite (pas de FK). -> purge `chunks` dans la transaction de `delete_note` /
  `delete_attachment` / `delete_attachments_for_note` (atomique, pas dans le thread embed). Sans ça,
  un futur reconstruct (T12) aurait ressuscité une note supprimée.
- **Édition <50 chars (MAJOR):** éditer une note sous le seuil laissait des chunks périmés. ->
  `purge_owner_chunks` sur l'early-return (count-guardé: n'ouvre LanceDB que si des BLOB existaient).

Différés (confirmés hors scope par la revue):
- doublons de vecteurs sur notes legacy ré-embarquées -> **T11** (backfill).
- divergence transitoire SQLite/LanceDB sur échec d'un des deux writes -> **T13** (reconcile boot).
- `content_hash` pour l'instant write-only -> consommé par T11/T13.

## Comment vérifier en local
```
cargo test -- --test-threads=1     # 158 verts, 11 ignorés (clés API). single-thread: table LanceDB partagée
make check                          # fmt + clippy: 0
make all                            # build iOS + install device
```
Tests clés:
- `tests/chunk_blob_test.rs`: BLOB f32 LE round-trip; N BLOB; re-embed atomique 0 orphelin;
  note+attachment indépendants en SQLite; **delete note purge note + ses attachments**; delete
  attachment purge ses chunks seulement.
- `tests/rag_integration_test.rs::test_note_and_attachment_chunks_coexist`: la régression BLOCKER 5
  (l'attachment survit au ré-embed de la note).

## Reste device-only (futur)
- Validation RAG bout-en-bout sur device (recherche après edit/delete) -> couverte par T24 (E2E).
- Reconstruction LanceDB depuis BLOB -> T12.
