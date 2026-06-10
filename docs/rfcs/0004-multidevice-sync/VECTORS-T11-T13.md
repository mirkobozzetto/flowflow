# Passe 4 - Vecteurs (T11 backfill, T12 reconstruct, T13 boot reconcile)

> But: fin de la chaîne vecteurs (variante B). SQLite `chunks` (BLOB f32) est la source de
> vérité; LanceDB devient un cache 100% reconstructible. Après cette passe: supprimer
> `vectordb/` -> reconstruction sans aucun appel d'embedding.

## Ce qui a été fait

### T11 - `backfill_legacy_chunks` (une fois)
- `reconcile.rs`: pour chaque note vivante, lit ses lignes LanceDB (`fetch_note_rows`) et copie le
  vecteur en BLOB SQLite avec id déterministe (`note:{id}:{idx}`; les `att:` gardent leur id).
- N'écrase jamais un owner déjà présent en SQLite (`count == 0`) -> préserve les données post-T10.
- SKIP les lignes d'un attachment supprimé (`get_attachment().is_some()`) -> pas de junk.
- Flag `chunks_backfilled_v10` posé seulement après succès complet (retry sûr sinon).
- `migrate_chunk_dates` (embed.rs + vectordb.rs) supprimé/fondu: une seule passe au boot.

### T12 - `reconstruct_from_blob` (chemin distinct, sans IA)
- Rebuild LanceDB depuis les BLOB SQLite. Parent d'un attachment résolu par join `get_attachment`.
- JAMAIS `ai.embed`, NON soumis au gate `ai_consent` (aucun appel réseau).
- `vectordb.rs`: `add_chunks` (insert-only create-or-add), `all_ids`, `delete_ids` (IN, batch 500),
  `fetch_note_rows` (décode FixedSizeList -> f32). Signature/schéma Arrow de `store_chunks` inchangés.

### T13 - `reconcile_once` + `run_boot_reconcile`
- Boot: UN thread, UN runtime, UN store -> pas de course LanceDB au démarrage (fix MAJOR RFC).
- Diff d'ids SQLite(truth) vs LanceDB: orphelins LanceDB supprimés, manquants reconstruits depuis BLOB.
- Self-heal (RFC §6 "orphelins -> suppression"): purge les chunks SQLite d'un owner (note/attachment)
  qui n'existe plus -> convergence garantie (aucun id ne reste coincé "missing"). Ne touche jamais
  une donnée vivante.
- Garde dimension: une ligne BLOB corrompue (dim != 1536) est sautée + loggée (pas de panic batch).
- Idempotent: 2e passe = no-op.

## Fixes issus de la revue adversariale (avant commit)
Revue ultracode 4 lentilles (16 agents): 12 findings, 7 "confirmés" par la vérif. Tri manuel:
- **Écartés (2):** un BLOCKER + un MAJOR "perte de données" décrivaient en fait la suppression du
  cache LanceDB d'une note/attachment **déjà supprimés par l'utilisateur** = nettoyage voulu (RFC §6).
  SQLite truth n'a plus ces chunks intentionnellement. Le jumeau "dismissed" de la revue le confirme.
- **Réels corrigés (3):**
  1. Attachment supprimé pendant un crash -> backfill ré-écrivait du junk SQLite non reconstructible
     (owner mort) -> reconcile bouclait sans converger. Fix: garde `get_attachment` au backfill +
     self-heal au reconcile.
  2. Tests fragiles en multi-thread (env var `FLOWFLOW_VECTORDB_PATH` globale). Fix: `Mutex` sérialisant.
  3. BLOB corrompu -> panic `chunks_to_batch`. Fix: garde dimension côté reconcile (CRITICAL
     `chunks_to_batch` non modifié).

## Comment vérifier en local
```
cargo test -- --test-threads=1     # 163 verts, 11 ignorés (clés API)
make check                          # fmt + clippy: 0
make all                            # build iOS + install device
```
Tests clés (`tests/reconcile_test.rs`, isolés via `open_at` + seam vectordb + Mutex):
- reconstruct rebuild note+attachment depuis BLOB; reconcile orphelins/manquants idempotent;
- backfill copie les vecteurs puis 0 id aléatoire restant; recovery (LanceDB vide) reconstruit sans IA;
- **deleted_attachment_leftovers_do_not_break_convergence** (régression du fix #1).

## Reste device-only (Mirko)
- Ouvrir l'app: créer/éditer une note, chat/RAG, supprimer une note avec attachment. Le
  backfill+reconcile tourne au boot sur la vraie base (additif, idempotent, flag une-fois).

## Suite (hors passe)
- Chaîne transport: T14 (Noise handshake/AEAD socket), T15 (appairage QR/IP + table sync_peers).
- At-rest T16 (NSFileProtection). Sync T17-T20/T23. E2E T24.
