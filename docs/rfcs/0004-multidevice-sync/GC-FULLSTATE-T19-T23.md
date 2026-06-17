# Passe 8 - GC + résurrection + full-state + exclusions (T19/T23)

RFC 0004, dernière passe de code. Le protocole sait désormais survivre aux
suppressions, aux backups restaurés et aux rappels jumeaux. Reste T24 (E2E
iPhone+Mac réels, validation Mirko).

## T19 - Tombstones: GC, add-wins, full-state

`services/sync/gc.rs` (nouveau), `protocol/{session,collect,apply,wire}.rs`,
`engine.rs`. Protocole bump v2 (HELLO gagne `next_seq`).

- **GC par acquittement**: un tombstone n'est purgé que quand TOUS les pairs
  ont prouvé l'avoir consommé. L'ack du pair sur MON espace de seq n'existait
  que sur le fil: chaque session le persiste (settings `sync_peer_acked_by_*`,
  écrasement - jamais de max - car un pair restauré régresse légitimement).
  Le GC tourne après chaque session réussie; `gc_horizon` = plus haut seq
  réellement purgé, annoncé dans HELLO.
- **Add-wins-over-delete**: un tombstone entrant (note/dossier/conversation)
  est VETOÉ si un enfant local vivant a été créé ici et jamais acquitté par le
  pair qui supprime (= ajout concurrent que le suppresseur ne pouvait pas
  connaître). Le parent est ré-authoré vivant (vv join + bump local domine le
  tombstone) et la résurrection repart vers le pair - jamais d'enfant orphelin,
  jamais de perte silencieuse. Les enfants que le suppresseur connaissait
  meurent normalement (leurs tombstones arrivent individuellement).
- **Full-state**: les deux côtés calculent le MÊME plan après l'échange HELLO
  (conditions symétriques): appareil restauré (le pair a acquitté plus que mon
  compteur n'a émis -> re-séquençage de mes méta au-dessus de son ack) ou ack
  régressé sous le `gc_horizon` du pair. La session pousse alors TOUTES les
  méta (par rowid, origines confondues, resumable). Le côté INTACT (autorité)
  applique la règle RFC "absent localement = supprimé": une ligne vivante sans
  méta locale est ARCHIVÉE dans `sync_conflicts` (texte + vecteurs, restaurable
  depuis l'UI) puis re-supprimée par tombstone dominant. Zéro résurrection,
  zéro perte définitive.

## T23 - Exclusions + rappels jumeaux

`protocol/apply.rs`, `db/note_reminder_repo.rs`.

- **Exclusions**: garanties STRUCTURELLES (ni trigger ni entrée catalogue pour
  `settings`/`pending_transcriptions`), désormais prouvées par test E2E: les
  clés API ne traversent jamais, l'état de job non plus.
- **Merge par intent**: un rappel distant dont le `(note_id, intent_hash)`
  existe localement sous un AUTRE id est le même rappel (jumeau). Vivant +
  jumeau actif -> on garde le local (id + handle OS device-local, pas de
  collision UNIQUE, pas de double notification). Vivant + jumeau annulé ->
  arbitrage HLC: le ré-ajout plus récent réactive le jumeau; l'annulation plus
  récente répond par un tombstone dominant (le zombie du pair meurt au lieu de
  ressusciter le rappel pour toujours).
- **Cancel cross-id**: le tombstone d'un rappel embarque son payload (la row
  soft-supprimée existe tant que la note vit) -> le récepteur annule son propre
  jumeau et ré-authore le changement (il se propage). Un cancel N'IMPORTE OÙ
  tue l'intent PARTOUT.
- **Ré-ajout sans crash**: `add_note_reminder` réactive une row jumelle
  soft-tombstonée au lieu de violer UNIQUE.

## Fix desktop (issue #20, commit séparé)

- `make desktop` / `make desktop-build`: Dioxus.toml filtré (awk) sans
  `[[ios.widget_extensions]]`, original parqué dans `.Dioxus.toml.ios` et
  restauré par trap. Le widget Live Activity ne compile plus jamais pour macOS.
- `db_path()`/audio/vectordb sur macOS: `~/Library/Application Support/
  FlowFlow` au lieu de temp_dir (purgeable). Migration NON destructive par
  copie (SQLite + .wav; l'index vectoriel se reconstruit des BLOBs, 0 appel
  API). Seam `FLOWFLOW_DATA_DIR` pour tests/instances secondaires.

## Tests (+7: `sync_gc_test.rs` 4, `sync_exclusions_test.rs` 3)

Suppression + GC + 3 syncs sans résurrection; GC bloqué sans ack / sans pair;
ajout d'enfant concurrent ressuscite le parent des 2 côtés (le vieil enfant
meurt, le nouveau survit); appareil restauré -> full-state: pas de
résurrection, N2 perdu revient, contenu re-supprimé archivé, compteur reseedé
(N3 post-restore atteint le pair); clés API/pending jamais transmises; cancel
cross-id; ré-ajout après cancel. Total suite: 203.
