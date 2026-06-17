---
feature: Synchronisation multi-appareils, LAN, sans serveur externe
slug: multidevice-sync
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
---

# PRD: Synchronisation multi-appareils (LAN, sans serveur externe)

## Problem statement

FlowFlow stocke tout en local, par appareil, dans le sandbox iOS (SQLite, LanceDB,
fichiers audio). La même app tourne sur iPhone et sur Mac (Apple Silicon), mais dans
deux sandboxes séparés qui ne partagent rien. Il n'existe aucune couche de sync: c'est
la seule raison de l'absence de synchro, pas une limite de fond.

Mirko veut une chose: retrouver toutes ses données sur chaque appareil, peu importe
lequel, sans jamais rien perdre, et hors-ligne. Deux règles non négociables encadrent
la solution: une seule solution claire, zéro perte de données, et rien sur un serveur
externe (aucun cloud tiers, la donnée reste chez lui).

Pourquoi maintenant: l'app est mûre (notes, RAG, chat, import, rappels) et installée sur
deux appareils; l'absence de sync est le frein principal à un usage réel multi-appareils.

## Goals

- Créer et éditer des notes sur n'importe quel appareil, hors-ligne, et tout retrouver
  partout après synchronisation.
- Garantir zéro perte de données, y compris en cas d'édition concurrente de la même note.
- Ne faire transiter aucune donnée par un serveur tiers: échange direct entre les
  appareils de Mirko, sur son réseau local.
- Garder les deux bases (SQLite + LanceDB) et embarquer chaque embedding une seule fois,
  sans re-payer le coût d'embedding sur chaque appareil.
- Ne prendre aucun risque sur les données existantes de la v1.0 (migration non destructive).

## Non-goals / Out-of-scope

- Pas de migration vers libSQL: elle n'apporte rien sans primary distant, et sa voie
  offline est en beta avec perte silencieuse possible. On reste sur rusqlite.
- Pas de serveur cloud, pas de Turso, pas de self-host externe (VPS): rien hors des
  appareils de Mirko. Aucune brique du PRD `lan-serve` (qui prévoit un relais VPS) n'est reprise.
- Pas de sync "depuis n'importe où" tant qu'aucun appareil n'est joignable. Sans serveur,
  la sync se fait quand les deux appareils se voient (même réseau).
- Pas de collaboration temps réel multi-utilisateurs.
- Pas de CRDT ni de merge ligne-à-ligne automatique de moteur tiers: la règle anti-perte
  est applicative et explicite.
- Android: plus tard.
- Sync à distance via réseau privé (type Tailscale) ou relais: option future, hors de ce PRD.

## Decisions (tranchées)

Chaque décision est arrêtée, avec sa justification et, le cas échéant, la source vérifiée
en ligne (2026). Le détail d'implémentation (schéma exact, protocole) appartient au RFC.
La revue adversariale a corrigé plusieurs hypothèses; les corrections sont intégrées ci-dessous.

**D1 - Pas de serveur externe: sync pair-à-pair sur le LAN.**
Un appareil expose un endpoint Rust quand l'app est au premier plan; l'autre s'y connecte
sur le même réseau et ils réconcilient. On ne reprend de `lan-serve` que le modèle de cycle
de vie "serveur foreground", PAS son réseau ni son relais VPS (interdit ici).
Justif: contrainte dure de Mirko (donnée chez lui uniquement).

**D2 - Pas de migration libSQL: on reste sur rusqlite.**
La valeur de libSQL était de synchroniser vers un primary distant (Turso Cloud ou sqld
self-host), que Mirko refuse. Sa voie offline (`new_synced_database`/`db.sync()`) est en
public beta depuis 2025-03-31, jamais GA, "data loss is possible". Rester sur rusqlite =
zéro migration = zéro risque sur les données v1.0.
Sources: https://turso.tech/blog/turso-offline-sync-public-beta , https://docs.turso.tech/libsql

**D3 - Source de vérité: le SQLite local de chaque appareil.**
Chaque appareil écrit toujours dans sa base locale (offline toujours possible, comme
aujourd'hui). La synchronisation est une réconciliation applicative entre les bases locales.

**D4 - Règle de conflit zéro-perte (applicative).**
Garantie produit: sur conflit, on garde une version courante ET on archive l'autre en
copie-de-conflit; rien n'est jamais jeté en silence. Suppressions propagées par tombstones.
POINT CRITIQUE (corrigé par la revue): la détection du conflit NE repose PAS sur `modified_at`
(horloge murale locale, sujette au décalage entre appareils, qui rendrait la détection
indécidable et autoriserait une perte silencieuse). Elle repose sur un **compteur de version
par ligne** + une **baseline** = la dernière version connue au sync précédent (watermark par
pair, par version, jamais par horloge). Si les deux côtés ont muté depuis la baseline commune
-> conflit -> copie-de-conflit systématique. `modified_at` ne sert qu'au tie-break d'affichage
"qui est courant", jamais à arbitrer la convergence. La copie-de-conflit réutilise le BLOB
vecteur de la source (zéro appel API) et porte un flag qui l'exclut d'un nouveau cycle
d'embedding et de détection de conflit. Mécanisme exact (compteur simple, vector clock ou HLC)
tranché au RFC.
Justif: aucun moteur ne fait le merge sans perte en turnkey (libSQL = last-push-wins
silencieux; LanceDB = pas de merge offline); la perte sous décalage d'horloge (LWW pur) est
un anti-pattern documenté.
Sources: https://docs.turso.tech/sync/conflict-resolution , https://github.com/lancedb/lancedb/issues/2319

**D5 - Vecteurs: variante B (BLOB dans SQLite), LanceDB reconstruit localement.**
Le(s) vecteur(s) sont stockés en BLOB dans SQLite et voyagent avec la sync. Sur les autres
appareils, LanceDB se remplit en copiant les octets depuis SQLite (zéro appel d'embedding).
LanceDB reste un index local reconstructible, jamais synchronisé. NB: BLOB brut, pas sqlite-vec.
Corrections de la revue: une note >50 chars produit PLUSIEURS chunks; l'identité de chunk doit
être **déterministe et stable** (`note:{note_id}:{idx}`, en miroir de l'attachment `att:{id}:{idx}`).
Aujourd'hui l'id de chunk de note est aléatoire (`Uuid::new_v4()`), à rendre déterministe avant
la sync. Au re-embed (N variable): purge atomique des anciens chunks puis réinsertion. Format
BLOB documenté (f32 little-endian, 1536 dims, même endianness arm64/arm64). La reconstruction
depuis BLOB N'EST PAS soumise au consentement IA (aucun appel réseau).

**D6 - Audio: compressé, fichier hors-base, transféré par le protocole de sync.**
L'audio reste un fichier hors-base (cohérent avec la migration V7 qui a sorti l'audio de la
table notes; évite de gonfler un `.db` ouvert en WAL sur le main thread). Il est compressé
puis transféré comme charge binaire par le protocole de sync (PAS stocké en BLOB SQLite), et
reconstruit en fichier local lisible côté récepteur.
Correction de la revue: il n'existe pas d'encodeur AAC/Opus pur Rust de production. Compression
AAC via AudioToolbox (FFI objc, cohérent avec le reste de `platform/ios`). Repli si on refuse
tout FFI: compression générique (zstd du PCM, ~2x au lieu de ~10x). Codec exact tranché au RFC
(cf. spike de faisabilité). Object storage exclu (externe).

**D7 - Appairage et sécurité.**
Appairage par code court / QR sur le LAN + clé pré-partagée. Transport **chiffré ET authentifié**
obligatoire: TLS avec épinglage d'empreinte du pair (pinning) / mTLS, ou Noise/PSK. Un chiffrement
sans authentification laisserait passer un MITM sur le Wi-Fi partagé. Découverte: QR / IP manuelle
en chemin PRIMAIRE (le mDNS pur Rust n'est PAS garanti sur iPhone réel: il exige l'entitlement
multicast restreint d'Apple, rarement accordé; mDNS via Network.framework objc envisageable en
option ultérieure).

**D8 - Périmètre de sync.**
Synchronisé: notes, folders, notes_folders, conversations, conversation_messages, attachments
(content_text), note_audios (+ audio compressé), note_reminders (intention), vecteurs.
Exclu: `pending_transcriptions` (état de job éphémère); `settings` et clés API (par appareil,
voir Open questions). Les rappels synchronisent l'intention (date, récurrence, `intent_hash`,
état; `note_reminders` a déjà un état `active`/`tombstone`) mais le `reminder_id`/`backend`
(handles OS EventKit/UserNotifications) sont ré-enregistrés localement sur chaque appareil.
Métadonnées de réconciliation par table (correction de la revue): entités mutables (notes,
folders, conversations, note_audios via `set_audio_transcription`, note_reminders) -> compteur
de version + tombstone; entités append-only / jonction (attachments, conversation_messages,
notes_folders) -> insert + tombstone (pas de `modified_at` inutile).

**D9 - Chiffrement au repos.**
Data protection iOS (NSFileProtection sur le `.db`), pas l'encryption libSQL (elle casse le
build iOS via cmake/AES). Source: https://github.com/tursodatabase/libsql/issues/1384

## RFC must resolve (gating avant tout code)

La revue a identifié des points de conception à trancher au RFC AVANT d'écrire le code; tant
qu'ils ne sont pas résolus, la garantie zéro-perte n'est pas tenable:

- **G1 (bloquant)** - Mécanisme de version par ligne + baseline/watermark par pair, remplaçant
  `modified_at` comme arbitre de conflit (compteur, vector clock ou HLC).
- **G2 (bloquant)** - Stratégie tombstone par entité ENFANT et sort des `ON DELETE CASCADE`
  existants (propager le tombstone aux enfants, ou retirer le CASCADE et cascader en applicatif).
- **G3 (bloquant)** - Schéma de chunk vecteur déterministe et uniforme; reconstruction
  depuis BLOB AVANT la boucle de réconciliation (jamais d'appel API si le BLOB existe).
- **G4 (majeur)** - Faisabilité (spikes): endpoint TLS pur-Rust cross-compile iOS; codec AAC
  via AudioToolbox FFI vs repli zstd; découverte QR/IP par défaut (mDNS non garanti device).

## User stories

1. **Créer partout.** En tant qu'utilisateur, je veux créer des notes sur l'iPhone et sur
   le Mac, même hors-ligne, afin de les retrouver toutes sur les deux après sync.
2. **Éditer et propager.** En tant qu'utilisateur, je veux éditer une note sur un appareil
   et voir la version à jour sur l'autre, afin de travailler indifféremment sur l'un ou l'autre.
3. **Conflit sans perte.** En tant qu'utilisateur, si j'édite la même note des deux côtés
   hors-ligne, je veux récupérer les deux versions, afin de ne jamais perdre une modification.
4. **Supprimer et propager.** En tant qu'utilisateur, je veux qu'une suppression sur un
   appareil se propage à l'autre (note ET ses enfants: attachments, audios, rappels), afin de
   ne pas voir réapparaître ce que j'ai effacé.
5. **Audio partout.** En tant qu'utilisateur, je veux que l'audio et sa transcription se
   synchronisent, afin d'écouter sur le Mac une note dictée sur l'iPhone.
6. **Recherche sans surcoût.** En tant qu'utilisateur, je veux que la recherche RAG marche
   sur chaque appareil sans re-payer l'embedding, afin que l'IA fonctionne partout gratuitement.
7. **Appairer simplement.** En tant qu'utilisateur, je veux appairer un nouvel appareil par
   un code/QR sur le réseau local, afin de lier mes appareils sans compte ni cloud.
8. **Offline d'abord.** En tant qu'utilisateur, je veux que tout marche hors-ligne et que la
   sync se fasse quand mes appareils se voient (y compris via un bouton "Sync maintenant"),
   afin de ne jamais être bloqué par le réseau.
9. **Donnée chez moi.** En tant qu'utilisateur, je veux qu'aucune donnée ne quitte mes
   appareils, afin de garder une confidentialité totale.
10. **Aucun risque v1.0.** En tant qu'utilisateur existant, je veux que la mise à jour ne
    touche pas mes données actuelles, afin de ne rien perdre en adoptant la sync.

## Acceptance criteria

**Story 1 (créer partout)**
- Given N notes créées hors-ligne sur l'iPhone et M sur le Mac, When les deux appareils
  synchronisent, Then chacun contient les N+M notes, 0 perdue, 0 dupliquée (pour les entités
  créées après l'activation de la sync; cf. dédup historique en Story 10).

**Story 2 (éditer et propager)**
- Given une note éditée sur un seul appareil, When la sync a lieu, Then l'autre appareil
  affiche la version éditée (contenu, titre, tags identiques), And aucune copie-de-conflit
  n'est créée (un seul côté a muté depuis la baseline).

**Story 3 (conflit sans perte)**
- Given la même note éditée différemment sur les deux appareils depuis la dernière baseline
  commune, When ils synchronisent, Then les deux appareils ont une version courante ET une
  copie-de-conflit contenant l'autre version, And aucune version n'est écrasée en silence,
  And ceci tient même si les horloges des deux appareils diffèrent.

**Story 4 (supprimer et propager)**
- Given une note (avec attachments, audios, rappels) supprimée sur un appareil, When la sync
  a lieu, Then la note ET tous ses enfants disparaissent sur l'autre appareil, And ne
  réapparaissent pas lors des syncs suivantes.

**Story 5 (audio partout)**
- Given une note dictée avec audio sur l'iPhone, When le Mac synchronise, Then l'audio est
  lisible sur le Mac et la transcription est présente, And la taille transférée est celle de
  l'audio compressé.

**Story 6 (recherche sans surcoût)**
- Given des notes embarquées sur l'iPhone, When le Mac les reçoit par sync et reconstruit son
  LanceDB depuis les BLOB, Then la recherche RAG renvoie les mêmes résultats, And 0 appel
  d'embedding (coût API = 0) n'a été émis par le Mac, And ceci fonctionne même si le
  consentement IA n'a pas été redonné sur le Mac.

**Story 7 (appairer simplement)**
- Given un appareil non appairé, When il présente une clé/empreinte invalide, Then la connexion
  est refusée et aucune donnée n'est exposée.
- Given un appareil appairé, When il se connecte sur le LAN, Then la sync démarre via un canal
  chiffré ET authentifié (empreinte du pair épinglée).

**Story 8 (offline d'abord)**
- Given un appareil hors-ligne, When je crée/édite/supprime des notes, Then tout fonctionne
  localement, And la sync s'effectue dès que l'autre appareil est joignable (auto à l'ouverture
  ou via "Sync maintenant").

**Story 9 (donnée chez moi)**
- Given une session de sync complète, When on inspecte le trafic réseau, Then 0 octet de
  données utilisateur n'est envoyé hors des appareils de Mirko (aucune requête vers un tiers).

**Story 10 (aucun risque v1.0)**
- Given une base v1.0 existante, When l'utilisateur met à jour vers la version avec sync,
  Then 100% de ses données sont présentes et intactes, And la migration est non destructive.
- Given des données historiques pré-sync (UUID audio backfillés indépendamment par appareil),
  When le premier appairage a lieu, Then une passe de dédup (match `note_id` + hash du contenu)
  évite les doublons d'audio, And aucune donnée n'est perdue dans la dédup.

## Success metrics

- 2 appareils, créations croisées hors-ligne: 0 note perdue, 0 doublon après sync (entités post-sync-ready).
- Édition concurrente de la même note: 100% des cas produisent 1 version courante + 1
  copie-de-conflit, 0 écrasement silencieux, y compris sous décalage d'horloge simulé (ex: +10 s).
- Suppressions (note + enfants): 100% propagées, 0 résurrection après syncs répétées.
- Vecteurs présents sur le 2e appareil avec 0 appel d'embedding (coût API à la réconciliation = 0).
- Reconstruction de LanceDB depuis les BLOB SQLite: index complet, 0 appel API, après suppression
  totale du dossier vectordb (sert aussi de recovery mono-appareil), même sans re-consentement IA.
- Sync d'un volume cible (500 notes + audio compressé) en moins de 60 s sur LAN.
- 0 octet de donnée utilisateur envoyé vers un serveur tiers (vérifié au niveau réseau).
- Données v1.0: 100% présentes après mise à jour (0 perte), migration idempotente.
- Validé sur iPhone réel (pas seulement simulateur).

## Constraints & assumptions

- 100% Rust, zero JS/TS, UI Dioxus, iOS d'abord. Exception assumée possible: encodage audio via
  AudioToolbox en FFI objc (pas d'encodeur pur Rust viable), à trancher au RFC (cf. D6/G4).
- On reste sur rusqlite (pas de libSQL). `notes`, `folders`, `conversations` ont déjà
  `modified_at`; la réconciliation ajoute un compteur de version par ligne et, par table, soit
  un tombstone+version (mutables) soit insert+tombstone (append-only/jonction).
- `note_reminders` a déjà un état `active`/`tombstone` (bon socle pour la propagation de suppression).
- Identité déjà par UUID sur toutes les entités (bon socle anti-collision), SAUF l'id de chunk
  vecteur de note (aléatoire aujourd'hui) à rendre déterministe.
- Serveur LAN joignable seulement quand l'app est au premier plan (limite iOS); la suspension
  coupe la sync. En usage solo, prévoir un déclencheur explicite (les deux apps rarement au
  premier plan en même temps).
- LanceDB reste un index local reconstructible, jamais source de vérité ni synchronisé.
- Format SQLite inchangé: les migrations ajoutent des colonnes/tables de sync, sans toucher
  aux données existantes (les migrations historiques V1-V9 ne sont jamais retouchées).
- La sync exige que les deux appareils soient joignables sur le même réseau (accepté par Mirko).

## Risks

- **Perte de données (CRITIQUE).** Le vecteur principal était l'arbitrage par horloge murale;
  neutralisé par le compteur de version + baseline (D4/G1) + copie-de-conflit systématique.
  Risque résiduel: prolifération de copies-de-conflit si l'édition concurrente est fréquente
  (rare en usage solo).
- **Résurrection d'entités enfants (CRITIQUE).** Les `ON DELETE CASCADE` effaceraient les enfants
  sans tombstone -> résurrection chez le pair. Doit être résolu entité par entité (G2).
- **Maturité / dépendance beta (écarté).** En abandonnant libSQL, on supprime la dépendance à un
  composant beta et à un binaire self-host figé. Source: https://github.com/tursodatabase/libsql/releases
- **Faisabilité Rust/iOS non prouvée (ÉLEVÉ).** Codec audio pur Rust inexistant (FFI AudioToolbox
  probable), mDNS pur Rust bloqué sans entitlement multicast Apple, TLS serveur cross-compile à
  valider. Mitigation: spikes avant implémentation (G4), repli QR/IP et zstd documentés.
- **Confidentialité LAN.** Sur Wi-Fi partagé, MITM possible si TLS non authentifié. Mitigation:
  empreinte épinglée / mTLS ou Noise/PSK obligatoires (D7).
- **Limite iOS foreground.** Sync seulement app ouverte; pas d'arrière-plan. Mitigation: "Sync
  maintenant" + sync à l'ouverture dès détection du pair.
- **Taille / perf DB.** Audio gardé hors-base (D6) pour ne pas pénaliser le `.db` (WAL, main thread).
- **Coût.** Nul: aucun serveur, aucun cloud. Embedding payé une seule fois (D5).
- **Croissance des tombstones.** GC seulement après acquittement par tous les pairs appairés
  (par version, pas par horloge), sinon résurrection (G2/RFC).

## Open questions

- Mécanisme de version exact (compteur simple vs vector clock vs HLC): tranché au RFC (G1).
- Stratégie tombstone x CASCADE par entité: tranché au RFC (G2).
- Codec audio: AudioToolbox FFI (qualité, ~10x) vs zstd pur Rust (~2x): spike puis RFC (G4).
- Clés API (`settings`): rester par appareil (défaut retenu) ou sync chiffrée optionnelle ?
- Rappels: ré-enregistrement automatique (hypothèse retenue par défaut) vs à la demande ?
- Découverte: QR/IP par défaut; mDNS via Network.framework objc plus tard, oui/non ?
- Résolution des copies-de-conflit dans l'UI: badge "conflit" différé vs prompt immédiat ?
- Sync à distance future (réseau privé type Tailscale): PRD séparé quand le besoin sera réel.
