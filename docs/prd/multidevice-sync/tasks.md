---
feature: Synchronisation multi-appareils, LAN, sans serveur externe
slug: multidevice-sync
type: tasks
source_prd: docs/prd/multidevice-sync/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Ne pas implémenter. Ceci est la liste de tâches dérivée. Lancer `ship` (ou
> l'implémenteur) pour exécuter. Un test concret est exigé à chaque étape, validé sur
> appareil réel quand c'est pertinent.
>
> Préalable: les points de conception G1-G4 du PRD (version par ligne, tombstone x CASCADE,
> schéma de chunk, faisabilité) doivent être tranchés au RFC avant tout code de la phase 6.0.

## Relevant Files

- `src/db/schema.rs` - migrations V10+ (version par ligne, tombstones par entité, table chunks déterministe).
- `src/db/mod.rs` - runner de migration, ouverture connexion (reste rusqlite, pas de libSQL - D2). `now_iso()` (modified_at = cosmétique uniquement).
- `src/db/note_repo.rs`, `folder_repo.rs`, `attachment_repo.rs`, `conversation_repo.rs`, `note_reminder_repo.rs` - incrémenter la version, poser tombstone, exposer le diff par version.
- `src/db/note_repo.rs:192` (`set_audio_transcription`) - doit incrémenter la version de note_audios.
- `src/services/embed.rs` - id de chunk déterministe; chemin "reconstruct from blob" sans `ai.embed` ni gate `ai_consent`; écrire le BLOB à l'embedding.
- `src/services/vectordb.rs` - reconstruire LanceDB depuis les BLOB SQLite (copie d'octets), purge des orphelins des deux côtés.
- `src/services/sync/` (nouveau) - réconciliation locale + protocole LAN push/pull + règle de conflit par version + copie-de-conflit.
- `src/platform/ios/` - AudioToolbox FFI (compression AAC), NSFileProtection, (option) Network.framework pour découverte.
- `src/ui/` - écran d'appairage (code/QR), bouton "Sync maintenant", indicateur de sync, affichage des copies-de-conflit.
- `docs/prd/lan-serve/prd.md` - référence du SEUL cycle de vie "serveur foreground" (ni réseau, ni relais VPS).

## Tasks

- [ ] 1.0 Fondations sync-ready: métadonnées de réconciliation _(PRD: D2, D4, D8, Story 10)_
  - [ ] 1.1 Garde D2: aucune introduction de libSQL; migrations rusqlite uniquement; ne jamais retoucher V1-V9.
  - [ ] 1.2 Migration V10: ajouter un compteur de version par ligne (INTEGER) sur les entités mutables
        (notes, folders, conversations, note_audios, note_reminders), incrémenté à chaque update.
  - [ ] 1.3 Métadonnées par table: append-only/jonction (attachments, conversation_messages,
        notes_folders) -> insert + tombstone, pas de `modified_at`. Mutables -> version + tombstone.
  - [ ] 1.4 Tombstones par entité (note ET enfants), avec stratégie explicite vis-à-vis des
        `ON DELETE CASCADE` (propager le tombstone aux enfants OU retirer le CASCADE + cascade
        applicative). Réutiliser l'état `active`/`tombstone` déjà présent sur note_reminders.
  - [ ] 1.5 `set_audio_transcription` incrémente la version de note_audios (sinon transcription non synchronisée).
  - [ ] Test: migrations sur une COPIE de la vraie base v1.0; 0 perte, l'app démarre; un update
        incrémente la version; supprimer une note AVEC attachment+audio+rappel pose un tombstone
        sur TOUS; idempotence (relancer la migration ne change rien).

- [ ] 2.0 Spikes de faisabilité (avant tout transport/audio) _(PRD: G4, contrainte 100% Rust)_
  - [ ] 2.1 Prouver un endpoint TLS pur-Rust (rustls) qui cross-compile aarch64-apple-ios + sim.
  - [ ] 2.2 Trancher la découverte: prouver QR/IP manuelle (défaut) suffisante; statuer sur mDNS
        (entitlement multicast Apple) -> repli IP par défaut documenté.
  - [ ] 2.3 Prouver la compression AAC via AudioToolbox (FFI objc) sur iOS et Mac; mesurer le
        ratio réel; sinon acter le repli zstd du PCM (~2x) et réviser la cible de D6.
  - [ ] Test: chaque spike produit un artefact qui BUILD et tourne sur appareil réel (TLS handshake
        OK, compression d'un échantillon, appairage QR), ou une décision écrite de repli.

- [ ] 3.0 Schéma de chunk déterministe + vecteur en BLOB (variante B) _(PRD: D5, G3, Story 6)_
  - [ ] 3.1 Table `chunks` uniforme: id logique déterministe (`note:{note_id}:{idx}` ou
        `att:{aid}:{idx}`), owner_id, owner_kind, idx, dim, vector BLOB (f32 LE, 1536), content_hash, version.
  - [ ] 3.2 Migrer l'id de chunk de note (aujourd'hui `Uuid::new_v4()` aléatoire) vers le schéma déterministe.
  - [ ] 3.3 Écrire le BLOB à l'embedding; au re-embed: DELETE atomique des chunks de l'entité puis re-INSERT (N variable, 0 orphelin).
  - [ ] 3.4 Backfill: embarquer une fois les notes/attachments existants et remplir les BLOB.
  - [ ] Test: créer une note multi-chunks -> N BLOB présents avec ids déterministes; éditer la note
        -> anciens chunks purgés, nouveaux insérés, 0 orphelin.

- [ ] 4.0 Reconstruction LanceDB depuis BLOB + réconciliation locale (idempotente) _(PRD: D5, G3, Story 6)_
  - [ ] 4.1 Chemin "reconstruct from blob" distinct de `embed_note`: copie d'octets, JAMAIS `ai.embed`
        si le BLOB existe, ET non soumis au gate `ai_consent`.
  - [ ] 4.2 Boucle de diff chunks attendus (SQLite) vs présents (LanceDB): manquants -> copie BLOB;
        orphelins -> suppression (des deux côtés); contenu changé -> re-indexation depuis BLOB.
  - [ ] 4.3 Lancer la boucle au démarrage et après chaque sync; garantir la convergence.
  - [ ] Test: supprimer entièrement le dossier `vectordb`, relancer; l'index se reconstruit depuis
        les BLOB, la recherche RAG remarche, 0 appel d'embedding émis (recovery + Story 6 prouvés),
        même sans re-consentement IA.

- [ ] 5.0 Transport LAN sécurisé: endpoint + appairage + chiffrement authentifié _(PRD: D1, D7, Story 7, Story 9)_
  - [ ] 5.1 Endpoint Rust foreground (cycle de vie inspiré de lan-serve, SANS son réseau ni relais VPS).
  - [ ] 5.2 Appairage par code court / QR sur le LAN: échange de la clé pré-partagée ET de l'empreinte du cert pair.
  - [ ] 5.3 Canal chiffré ET authentifié: TLS avec pinning d'empreinte / mTLS (ou Noise/PSK); rejet
        de toute connexion sans empreinte+clé valides.
  - [ ] 5.4 Découverte: QR / IP manuelle par défaut (mDNS optionnel ultérieur selon spike 2.2).
  - [ ] Test: appairer iPhone et Mac; une clé OU une empreinte invalide est refusée; le canal est
        chiffré et authentifié (test "empreinte modifiée -> refus"); capture réseau: 0 connexion tierce.

- [ ] 6.0 Protocole de réconciliation inter-appareils (zéro perte par version) _(PRD: D4, G1, G2, Story 1-4, Story 8)_
  - [ ] 6.1 Watermark par pair et par version (jamais par horloge); push/pull des lignes mutées depuis la baseline.
  - [ ] 6.2 Règle de conflit: absent -> ajouté; muté d'un seul côté depuis la baseline -> propagé;
        muté des deux côtés -> version courante + copie-de-conflit. La copie-de-conflit COPIE le BLOB
        vecteur de la source (0 appel API) et porte un flag l'excluant de l'embedding et d'une nouvelle détection.
  - [ ] 6.3 Tombstones: propagation des suppressions (note + enfants), 0 résurrection; GC d'un
        tombstone uniquement quand tous les pairs ont acquitté une version >= celle du tombstone.
  - [ ] 6.4 Déclencheurs: bouton "Sync maintenant", sync à l'ouverture dès détection du pair, à la
        sauvegarde (debounced).
  - [ ] Test: les 3 scénarios sur 2 appareils -> (a) inserts croisés: tout présent, 0 doublon;
        (b) édition d'un seul côté: propagée, 0 copie-de-conflit; (c) édition des deux côtés AVEC
        horloges décalées (+10 s sur un appareil): version courante + copie-de-conflit, 0 écrasement
        silencieux. Suppression note+enfants: 0 résurrection après 3 syncs.

- [ ] 7.0 Synchronisation de l'audio _(PRD: D6, G4, Story 5)_
  - [ ] 7.1 Compression de l'audio (AAC via AudioToolbox, ou repli zstd selon spike 2.3), fichier hors-base.
  - [ ] 7.2 Transfert du fichier compressé comme charge binaire du protocole de sync (pas de BLOB SQLite).
  - [ ] 7.3 Reconstruction du fichier local lisible côté récepteur après sync.
  - [ ] 7.4 Dédup historique au premier appairage: audios pré-sync (UUID backfillés indépendamment)
        dédupliqués par `note_id` + hash du contenu, 0 perte.
  - [ ] Test: dicter une note avec audio sur iPhone; après sync, audio lisible sur Mac, transcription
        présente, taille transférée = audio compressé; au premier appairage, 0 doublon d'audio historique.

- [ ] 8.0 Cas spéciaux et exclusions _(PRD: D8)_
  - [ ] 8.1 Exclure `pending_transcriptions` (état de job propre à l'appareil).
  - [ ] 8.2 `settings`/clés API par appareil (non synchronisés) - voir Open question.
  - [ ] 8.3 Rappels: synchroniser l'intention (date, récurrence, `intent_hash`, état) et ré-enregistrer
        localement le `reminder_id`/`backend` (hypothèse retenue: automatique; cf. Open question).
  - [ ] Test: créer un rappel sur iPhone; après sync, ré-enregistré sur Mac sans doublon (`intent_hash`);
        les clés API ne traversent pas; `pending_transcriptions` ignoré.

- [ ] 9.0 Chiffrement au repos _(PRD: D9)_
  - [ ] 9.1 Activer/vérifier NSFileProtection sur le `.db` (notes + vecteurs BLOB) et le dossier audio.
  - [ ] 9.2 Confirmer qu'aucune encryption libSQL n'est utilisée (build iOS OK, pas de cmake/AES).
  - [ ] Test: fichier `.db` inaccessible appareil verrouillé (protection active); build iOS passe sans erreur AES.

- [ ] 10.0 Validation bout-en-bout et garanties zéro-perte _(PRD: Success metrics, Story 9, Story 10)_
  - [ ] 10.1 Scénario complet sur iPhone + Mac réels: créations croisées, éditions, conflit (horloges
        décalées), suppression note+enfants, audio, recherche RAG.
  - [ ] 10.2 Mesurer chaque métrique chiffrée du PRD: 0 perte, 0 doublon (post-sync-ready), 0 appel
        d'embedding sur le 2e appareil, sync < 60 s pour 500 notes + audio compressé, 0 octet vers un tiers.
  - [ ] 10.3 Rejouer la mise à jour depuis une base v1.0 réelle: 100% des données intactes + dédup historique OK.
  - [ ] Test: rapport de validation avec toutes les métriques au vert (pass/fail sur les seuils chiffrés), sur appareil réel.
