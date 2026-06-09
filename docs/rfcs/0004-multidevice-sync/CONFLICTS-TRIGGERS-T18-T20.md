# Passe 7 - Conflits + Déclencheurs (T18/T20)

RFC 0004. La sync devient UTILISABLE: l'app écoute, pousse, montre son état
et ses conflits. Plus aucun module dormant.

## T18 - Politique de merge + conflits visibles

`services/sync/conflict.rs` (nouveau), `services/sync/vv.rs` (algèbre VV
extraite de protocol/), `db/conflict_repo.rs` (nouveau), `ui/sync/conflicts.rs`.

- `decide()` pur et testable: dominance VV -> skip/apply; concurrence ->
  gagnant déterministe `(updated_hlc, origin_device)` (identique des 2 côtés);
  VV corrompu -> conflit forcé avec flag par côté (local = DB endommagée,
  distant = pair cassé). Matrice de 7 cas unitaires.
- `losing_vector_ref` REMPLI: les chunks BLOB du perdant (payload distant, ou
  table locale lue AVANT que le gagnant n'écrase) sont archivés en JSON dans
  `sync_conflicts`. Restaurer un conflit ne coûte AUCUN appel API.
- Résolution UI (section "Conflits" de l'écran Sync): "Restaurer en note"
  (nouvelle note synchronisée + vecteurs recopiés sous ids déterministes +
  bump méta -> tout repart vers les pairs) ou "Ignorer" (resolved=1, la
  donnée reste archivée en base - jamais détruite).
- Restore durci par la revue: claim-first (`resolved 0->1` gardé) = un
  double-tap ou une liste périmée ne duplique jamais; échec de création =
  claim rollback (le conflit reste visible); chunk illisible = skip + log
  (la note se ré-embeddera à la prochaine édition).

## T20 - Déclencheurs + indicateur

`services/sync/engine.rs` (nouveau), `ui/sync/{mod,controls}.rs`,
`platform/ios/sync_ffi.rs`, hooks dans detail/menu/transcription/embed.

- `SyncEngine` au boot de l'app: listener foreground (port 53127, seam
  `FLOWFLOW_SYNC_PORT`), sync à l'ouverture, bouton "Synchroniser
  maintenant", debounce 3 s après sauvegarde/suppression/transcription/embed.
- Aucune requête perdue: chaque trigger lève `pending`; le détenteur du lock
  re-passe tant qu'une requête est arrivée après son dernier snapshot.
- Carnet d'adresses par pair (settings `sync_peer_addr_*`): seedé à
  l'appairage des deux côtés, rafraîchi à chaque session servie (DHCP).
- Listener auto-réparant: les erreurs accept (socket iOS défunt après
  suspension) ne touchent pas le statut et déclenchent un re-bind après 3
  échecs - l'inbound survit aux background/foreground.
- Indicateur TOUJOURS honnête: Syncing / Done (heure locale, ↑poussées
  ↓appliquées, conflits) / Error (message conservé à l'écran) / échec
  partiel multi-pairs affiché même quand un autre pair a réussi.
- `beginBackgroundTaskWithExpirationHandler` (FFI dynamique, thread-safe)
  autour des passes sortantes: fenêtre de grâce au background, end-once
  garanti même si l'expiration gagne la course au begin.
- Reconcile post-apply: les BLOBs reçus alimentent LanceDB sans attendre le
  boot -> le RAG du 2e appareil fonctionne dès la session suivante.

## Revue adversariale (ultracode, 4 lentilles, 37 agents, 2.43M tokens)

28 findings confirmés (~12 racines), 5 réfutés. Corrigés avant commit:
restore atomique (claim-first + rollback + best-effort), bump méta des
vecteurs restaurés (sinon jamais collectés), pending flag debounce, poke
engine après embed (sinon vecteurs frais échoués + push chunkless pouvant
effacer les chunks du pair), re-bind listener, échec partiel visible, heure
locale, course begin/expiration bg task, log corruption par côté.

## Tests (+13: `sync_conflict_test.rs` 9, `sync_engine_test.rs` 4)

Matrice decide() (égal/dominances/tie-break ±10s/device-id/corruption),
perdant archivé AVEC ses vecteurs + enfants intacts (attachment/audio/
dossier), restore -> texte + vecteurs + propagation au pair à la session
suivante, double-restore refusé, dismiss, carnet d'adresses (seed/clear/
refresh réel), sync manuelle E2E par l'engine, debounce qui collapse et
livre, pair injoignable = erreur VISIBLE. Total suite: 196.

## Ce que ça NE fait PAS encore

T19 (GC tombstones + add-wins-resurrect + full-state), T23 (exclusions +
merge rappels cross-id), T24 (E2E iPhone+Mac réels mesuré). Multi-instance
même machine: ports distincts non découverts (seam dev, documenté).
