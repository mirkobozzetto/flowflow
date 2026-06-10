---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
stack: "rust / cargo"
generated: "2026-06-10"
ran_by: "user"
pass_scope: "T19/T23 (GC + full-state + exclusions) + fix desktop #20 - passes 1-7 archivées dans trace.md"
---

# Verification Bundle: GC + full-state + exclusions (T19/T23) + desktop

> ship ne lance PAS ces commandes (ta règle: tests/builds = toi). La chaîne de
> passe pré-autorisée a déjà fait tourner tests + fmt + clippy + make all +
> make desktop-build en vert; ce bundle te permet de REJOUER les preuves.

## Safe checks

| Command | Validates | Expected pass signal |
|---------|-----------|----------------------|
| `cargo test --test sync_gc_test -- --test-threads=1` | C18: 0 résurrection après GC + 3 syncs, add-wins ressuscite le parent, appareil restauré -> full-state sans perte | `4 passed` |
| `cargo test --test sync_exclusions_test -- --test-threads=1` | C20: clés API/pending_transcriptions ne traversent pas, cancel cross-id, ré-ajout après cancel | `3 passed` |
| `cargo test --test sync_protocol_test -- --test-threads=1` | non-régression T17 après protocole v2 | `11 passed` |
| `cargo test -- --test-threads=1` | régression globale | `203 passed` |
| `make check` | fmt + clippy (lib, features mobile) | exit 0 |
| `make desktop-build` | fix #20: build macOS SANS compilation du widget Swift, Dioxus.toml restauré après | `Client build completed` + `git diff Dioxus.toml` vide |

## Device / réel (USER ONLY - prélude à T24)

1. `make all` sur l'iPhone; `make desktop` sur le Mac (même Wi-Fi).
2. Au premier lancement Mac: log `[db] migrated N legacy files` si tu avais
   des données desktop en temp; la base vit désormais dans
   `~/Library/Application Support/FlowFlow/`.
3. **Suppression**: supprime une note sur l'iPhone -> après sync elle
   disparaît du Mac, et n'y REVIENT PAS après 2-3 syncs supplémentaires.
4. **Add-wins**: coupe le Wi-Fi; supprime une note sur l'iPhone; sur le Mac,
   importe un document dans la MÊME note; rallume + sync (2 passes) -> la note
   est VIVANTE des deux côtés avec le nouveau document.
5. **Rappels jumeaux**: crée le même rappel sur les 2 appareils, sync (1 actif
   chacun, pas de double notif), annule-le sur un seul -> après sync il est
   annulé sur l'autre aussi.
6. Non-régression: notes, audio, RAG, conflits (section Conflits).

## Contract coverage (cette passe)

- C18 (suppression note+enfants: 0 résurrection après 3 syncs; ajout enfant
  concurrent ressuscite le parent; appareil restauré -> full-state) ->
  `sync_gc_test` (4 tests dont restore par copie de fichier réelle) +
  device 3/4.
- C20 (clés API ne traversent pas; pending_transcriptions ignoré; rappel
  syncé sans collision UNIQUE ni double notification) ->
  `sync_exclusions_test` + `sync_protocol_test::same_reminder_intent_*` +
  device 5.

Hors de cette passe: C21 (T24 E2E iPhone+Mac mesuré - protocole remis à
Mirko dans trace.md).
