---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
stack: "rust / cargo"
generated: "2026-06-10"
ran_by: "user"
pass_scope: "T18/T20 (conflits + déclencheurs) - passes 1-6 archivées dans trace.md"
---

# Verification Bundle: Conflits + déclencheurs (T18/T20)

> ship ne lance PAS ces commandes (ta règle: tests/builds = toi). Lance-les
> toi-même. Chaque ligne dit ce qu'elle prouve. NB: la chaîne de passe
> pré-autorisée a déjà fait tourner tests + fmt + clippy + make all en vert;
> ce bundle te permet de REJOUER les preuves.

## Safe checks

| Command | Validates | Expected pass signal |
|---------|-----------|----------------------|
| `cargo test --test sync_conflict_test -- --test-threads=1` | C17: matrice decide(), perdant archivé avec vecteurs, enfants intacts, restore + propagation, double-restore refusé | `9 passed` |
| `cargo test --test sync_engine_test -- --test-threads=1` | C19 (côté host): triggers, carnet d'adresses, debounce, erreur visible | `4 passed` |
| `cargo test --test sync_protocol_test -- --test-threads=1` | non-régression T17 après extraction du merge | `11 passed` |
| `cargo test -- --test-threads=1` | régression globale | `196 passed` |
| `make check` | fmt + clippy (lib, features mobile) | exit 0 |

## Device / réel (USER ONLY - C19 se valide ICI)

1. `make all` sur l'iPhone + `make desktop` sur le Mac (même Wi-Fi).
2. Appairage (si pas déjà fait): Settings -> Synchronisation -> "Afficher un
   code" sur l'un, coller l'URI sur l'autre.
3. **Sync à la sauvegarde**: crée/édite une note sur un appareil, les deux
   apps au premier plan -> elle apparaît sur l'autre ~3 s après la sauvegarde.
4. **Bouton**: "Synchroniser maintenant" -> indicateur passe Syncing puis
   "Synchronisé à HH:MM · ↑n ↓m".
5. **Conflit**: coupe le Wi-Fi, édite la MÊME note des deux côtés, rallume,
   re-sync -> même version gagnante des deux côtés + section "Conflits" avec
   la version écartée. "Restaurer en note" la ramène (et elle se propage).
6. **RAG sans re-embedding**: pose une question dont la réponse est dans une
   note créée sur L'AUTRE appareil -> trouvée, et le log montre
   `[reconcile] +n missing` (copie BLOB), AUCUN appel embedding.
7. **Stall visible**: éteins le Mac, "Synchroniser maintenant" sur l'iPhone
   -> erreur affichée (pas de silence).
8. Non-régression: notes, audio, suppression, rappels.

## Contract coverage (cette passe)

- C17 (édition des 2 côtés ±10s -> 1 courant enfants intacts + 1 entrée
  sync_conflicts, 0 écrasement) -> `sync_conflict_test` (matrice +
  `conflict_archives_losing_vectors_and_keeps_children_intact`) + device 5.
- C19 (sync dès détection du pair; indicateur visible; fenêtre de grâce au
  background) -> `sync_engine_test` + device 3/4/7; la fenêtre de grâce
  (beginBackgroundTask) ne se prouve que sur device: backgrounder l'app
  pendant un gros transfert -> il se termine ou reprend sans perte.

Hors de cette passe: C18 (T19 GC/résurrection), C20 (T23 exclusions), C21
(T24 E2E mesuré).
