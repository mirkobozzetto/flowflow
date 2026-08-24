---
proposal_id: "0002"
slug: "collaborative-shared-folders"
title: "Espaces collaboratifs : dossiers partagés (RFC 0027)"
status: Accepted
format: full
author: "Mirko Bozzetto"
created: "2026-08-24"
updated: "2026-08-24"
stepsCompleted: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
resume_cmd: "/ship docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
scope_path: "/Users/mirkobozzetto/code/flowflow"
auto_mode: false
skip_review: false
source_brief: "docs/brief/shared-folders/brief.md"
issue: "marketplace-flowflow#88"
---

# 0002 : Espaces collaboratifs, dossiers partagés (RFC 0027)

## 1. Résumé

Le partage livré ne diffuse qu'un instantané: republier change le code,
un dossier ne se partage pas, et rien ne descend aux sous-dossiers. Une
équipe n'a aucun espace commun vivant.

Ce document tranche trois choses: un plan `spaces` côté backend avec
autorité serveur et lecture par curseur, un mode de dossier `read` ou
`collab` dont le droit réel est le plus restrictif de la chaîne
d'ancêtres, et une suppression qui voyage dans le même flux de delta
jusqu'à la purge de l'index vectoriel.

Impact: une migration backend, une migration app V26, un module
d'application nouveau, et une note reçue qui reste une note ordinaire,
donc indexée et utilisable en chat sans code supplémentaire. Le plan
P2P du cluster et le plan `shares` existant ne bougent pas.

## 2. Contexte et code existant

### Le plan de partage existe déjà (proposal 0001, livré 2026-08-23)

App, `src/`:

- `domain/share.rs`: `ShareKind` (`note` | `thread`), `LocalShare`
  (source_id, code, expires_at), `Provenance` (état `live` | `gone`),
  lien `flowflow://share/{code}`.
- `application/sharing.rs`: `publish_note`, `publish_thread`, `revoke`,
  `open`, `append`, `update_own`, `delete_own`, `report`,
  `delete_my_notes`, `keep_note`, `align_kept_content`. Le partage
  d'un thread pousse un instantané des notes (`PublishNote {title,
  content}`), il ne diffuse rien après coup.
- `application/share_inbox.rs`: la boîte de réception d'un code.
- `persistence/share_repo.rs` + `schema.rs` V25: `note_shares`,
  `note_provenance`. Sans clé étrangère, par contrainte de l'applicateur
  de sync (`INSERT OR REPLACE`), nettoyage manuel dans le chemin de
  suppression.
- `application/note_persistence.rs::delete_note`: supprime audio,
  note, provenance, share, et appelle `embed::delete_note_embeddings`.
- `infrastructure/vectordb.rs`: `delete_ids`, `delete_note_chunks`,
  `delete_note_own_chunks`, `delete_attachment_chunks`.

Backend `marketplace-flowflow`, `src/features/shares/`:

- Tables (migration 17): `shared_threads` (code, web_user_id,
  account_id, kind, source_id, expires_at, revoked_at), `shared_notes`
  (thread_code, author_web_user_id, content, deleted_at),
  `share_reports`.
- Routes: `POST /v1/shares`, `/read`, `/append`, `/note/update`,
  `/note/delete`, `/revoke`, `/report`, `/delete-my-notes`; plan admin
  `/v1/admin/share-reports`, `/shares/read|revoke|revoke-all`.
- `publish` exige un `web_user` lié, premium (`gate::is_premium`), une
  expiry obligatoire plafonnée (`MAX_EXPIRY_DAYS`) et un quota
  (`MAX_ACTIVE_SHARES`). Republier tue le code précédent.
- `read` est ouvert à tout appareil authentifié qui a le code; les
  drapeaux `own` disent ce que l'appelant peut éditer.

### Les dossiers aujourd'hui

- `domain/folder.rs`: `Folder {id, name, description, parent_id}`,
  `flatten_tree`, `subtree_ids`. Aucune notion de mode ni de
  propriétaire.
- `persistence/folder_repo.rs`: CRUD + `notes_folders` (N:N),
  `folders_for_note`.
- `ui/sidebar/folders.rs`, `ui/notes/folder_picker.rs`,
  `ui/notes/row_menu.rs`: renommer, déplacer, supprimer.

### Deux plans de transport coexistent déjà

- `infrastructure/sync/`: P2P entre les appareils d'UN compte (Noise,
  vecteurs de version `vv.rs`, tombstones, `engine.rs`, `catalog.rs`,
  `conflict.rs`, `gc.rs`). Autorité distribuée, pas de serveur.
- `infrastructure/backend/shares.rs`: HTTP vers le backend, autorité
  serveur, lecture à la demande par code.

Le partage entre personnes passe donc par le backend, jamais par le
plan P2P: celui-ci est réservé aux appareils d'un même compte.

### Identité

- `application/profile.rs` + backend `features/profile/`: champs de
  profil avec visibilité par champ (dont une pastille `groups` non
  encore réelle), avatar. `authorship.rs`, `notes.author_device` (V23).

### Prior art

- `docs/proposals/0001-person-profiles-...`: profils + notes/threads
  partagés. Le partage de dossier en était explicitement exclu.
- `docs/rfcs/0025-identity-and-customer-account-site`: compte client,
  `account.flowflow.be`, nommage d'appareil.
- `docs/rfcs/0026-premium-account-spread-ux`: premium et propagation
  au cluster.
- `docs/brief/shared-folders/brief.md`: le brief produit de ce document.
- Issue `marketplace-flowflow#88`.

### Chemins d'exécution touchés

- Publication: UI menu de ligne -> `sharing::publish_*` ->
  `POST /v1/shares` -> code -> `note_shares`.
- Lecture: code -> `sharing::open` -> `POST /v1/shares/read` ->
  `share_inbox` -> `keep_note` -> `embed_note` -> LanceDB.
- Alignement: `align_kept_content` relit chaque code, supprime la copie
  locale et ses embeddings sur tombstone, grise la provenance sur 404.
- Suppression: `delete_note` -> `delete_note_embeddings` -> LanceDB.

## 3. Problème et motivation

Le partage livré couvre une note ou un thread: un instantané poussé une
fois, relu par code, sans vie propre. `publish_thread` sérialise les
notes au moment de l'appel; rien de ce qui est ajouté ensuite
n'atteint le lecteur tant que l'auteur ne republie pas, et republier
change le code.

L'unité de travail de FlowFlow est le dossier. Une équipe qui veut un
espace commun n'a aujourd'hui que le partage note par note, refait à
chaque ajout, sans droit d'écriture, sans révocation ciblée, sans
héritage de permission.

Trois questions ne se laissent pas trancher pendant l'implémentation,
parce que chacune fige des données chez plusieurs personnes:

1. Où vit l'autorité sur le contenu d'un espace, et selon quel modèle
   de permission descendant aux sous-dossiers.
2. Comment une note ajoutée par un membre atteint les autres sans
   interroger le serveur en continu.
3. Comment une suppression traverse le backend, les appareils, et
   l'index vectoriel de chacun, sans laisser de note fantôme
   interrogeable en chat.

La troisième porte un risque juridique: une note effacée qui ressort
dans une réponse RAG chez un autre membre n'est pas effacée.

## 4. Objectifs et non-objectifs

### Objectifs

- Un modèle de permission par dossier, `read` ou `collab`, hérité par
  les sous-dossiers, opposable côté serveur et pas seulement dans
  l'interface.
- Un espace vivant: une note ajoutée par un membre arrive chez les
  autres sans republication ni changement de code.
- Un budget serveur borné: la fraîcheur se paie en delta, pas en
  interrogation continue.
- Une suppression qui traverse backend, appareils et index vectoriel,
  vérifiable.
- Une sortie qui laisse au partant une copie locale de ses notes.
- Un seul plan de transport pour le contenu partagé, distinct du P2P
  du cluster, sans double autorité sur une même note.

### Non-objectifs

- Onboarding et création de compte depuis l'app: tâche 1 du brief,
  hors de ce document.
- Interface d'invitation, lien universel, QR code: tâche 3 du brief.
- Rôles fins et délégation d'invitation.
- Édition d'une note dont on n'est pas l'auteur.
- Chiffrement de bout en bout de l'espace.
- Fusion d'un dossier local existant dans un espace.
- Android, Windows, Linux. La conception ne doit rien leur fermer.

## 5. Alternatives envisagées

Trois axes indépendants. Chacun garde le statu quo comme option.

### Axe A: où vit l'autorité sur le contenu d'un espace

| Option | Ce que c'est | Pourquoi non |
|---|---|---|
| A0. Statu quo | Republier un thread à chaque ajout | Le code change, le lecteur perd le fil, rien n'est vivant |
| A1. Un espace = un `shared_thread` par dossier | Réutilise les 8 routes existantes | Pas d'arbre, pas de permission, quota par code, contenu figé au `publish` |
| A2. Nouveau plan `spaces`, autorité serveur, lecture par delta | Tables `spaces`, `space_members`, `space_folders`, `space_notes`; le client tire ce qui a changé depuis un curseur | Retenu |
| A3. Étendre le P2P aux appareils d'autres comptes | Pas de serveur, chemin e2e ouvert | iOS n'a pas de serveur entrant persistant; exige les deux appareils présents en même temps; la révocation devient invérifiable |
| A4. CRDT répliqué complet | Convergence sans autorité | Aucune écriture concurrente sur une même note dans ce produit: on paie une machinerie pour un conflit qui n'existe pas |

A3 est la seule qui offrirait le chiffrement de bout en bout. Elle
échoue sur la contrainte plateforme déjà constatée: pas de serveur
entrant persistant sur iOS.

### Axe B: modèle de permission et héritage

| Option | Ce que c'est | Pourquoi non |
|---|---|---|
| B1. Mode copié à la création | L'enfant hérite du mode du parent une fois pour toutes | Change le parent en lecture seule et les enfants restent ouverts: la restriction ne descend pas |
| B2. Mode déclaré + mode effectif = le plus restrictif de la chaîne | Chaque dossier déclare `read` ou `collab`; le droit réel se résout en remontant jusqu'à la racine de l'espace | Retenu |
| B3. ACL par membre et par dossier | Granularité complète | Le brief exclut les rôles en v1; une ACL sans interface de rôles est une base de données qu'on ne peut pas administrer |
| B4. Permission portée par le membre, pas par le dossier | Un membre est lecteur ou contributeur de tout l'espace | Contredit le besoin: un dossier lecture seule dans un espace collaboratif |

B2 rend la règle « un parent lecture seule n'accepte aucun enfant
collaboratif » vraie par construction plutôt que par validation à la
saisie, et survit au déplacement d'un sous-arbre.

### Axe C: propagation d'une suppression jusqu'à l'index vectoriel

| Option | Ce que c'est | Pourquoi non |
|---|---|---|
| C0. Statu quo `align_kept_content` | Relire chaque code, comparer note par note | Relit tout le contenu à chaque passe: le coût croît avec le corpus, pas avec les changements |
| C1. Journal de tombstones par espace, consommé par curseur | Une suppression écrit une ligne; le client applique le delta, purge SQLite puis LanceDB | Retenu |
| C2. Masquage local sans suppression | Simple, réversible | Une note masquée reste dans l'index: elle ressort en chat. Défaut de conformité, pas un compromis |
| C3. Purge par réindexation complète de l'espace | Sûr | Recalcule des embeddings payants pour supprimer une note |

C1 partage son curseur avec l'axe A2: une suppression est un
changement comme un autre dans le flux de delta.

## 6. Conception retenue

A2 + B2 + C1. Un plan `spaces` côté backend, autorité serveur, tiré
par curseur. Côté app, une note reçue est une vraie note locale: elle
traverse le pipeline d'indexation existant sans code nouveau.

### 6.1 Modèle backend

Nouvelles tables dans `marketplace-flowflow`, migration 18:

```
spaces(id, account_id, owner_web_user_id, name, created_at,
       revoked_at)
space_members(space_id, web_user_id, joined_at, removed_at)
space_folders(id, space_id, parent_id, name, mode, author_web_user_id,
              seq, created_at, updated_at, deleted_at)
space_notes(id, space_id, folder_id, author_web_user_id, title,
            content, seq, created_at, updated_at, deleted_at)
space_invites(code, space_id, created_by, expires_at, consumed_at,
              consumed_by)
```

`mode` vaut `read` ou `collab`. `parent_id` NULL = racine de l'espace.

Un code d'invitation est à usage unique: `consumed_at` posé au premier
`join` réussi le rend inerte. Réintégrer quelqu'un demande un nouveau
code, ce qui laisse une trace de qui a fait entrer qui. Un membre
révoqué qui rejoint plus tard réutilise sa ligne `space_members`
(`removed_at` remis à NULL) et repart au curseur 0.

`seq` est un entier monotone par espace, incrémenté à chaque écriture,
y compris une suppression. Il est attribué par un compteur porté par la
ligne `spaces`, dans la MÊME transaction que l'écriture
(`UPDATE spaces SET seq_counter = seq_counter + 1 ... RETURNING`), puis
posé sur la ligne modifiée. Le pool sqlx ouvre plusieurs connexions:
sans ce verrou par transaction, deux écritures concurrentes peuvent
recevoir le même numéro et un appareil saute un changement. Il n'y a pas de table d'événements: la
ligne modifiée porte son propre numéro d'ordre, et une ligne
`deleted_at` non nulle EST la tombstone. Un curseur suffit à décrire
ce qu'un appareil a déjà vu.

Une note supprimée garde sa ligne avec `title` et `content` mis à
NULL: la tombstone doit survivre pour atteindre un appareil hors ligne,
son contenu non.

### 6.2 Permission effective

```
effective_mode(f) = collab  si  f.mode = collab
                    et pour tout ancêtre a de f: a.mode = collab
                    sinon read
```

Résolue côté serveur à chaque écriture, jamais seulement dans
l'interface. Trois règles la complètent:

- Le propriétaire de l'espace écrit partout.
- Un membre écrit dans un dossier dont le mode effectif est `collab`.
- Un membre ne modifie ni ne supprime que ses propres notes, quel que
  soit le mode.

Conséquence voulue: rendre un dossier `read` restreint tout son
sous-arbre immédiatement, sans réécrire les enfants. Déplacer un
sous-arbre sous un parent `read` le restreint de la même façon.

Deux garde-fous, parce que la résolution remonte la chaîne à chaque
écriture: un plafond de profondeur (8 niveaux) borne le coût et rend
un cycle impossible à ignorer silencieusement; un déplacement sous un
de ses propres descendants est refusé, comme `subtree_ids` le fait
déjà côté app. Supprimer un dossier tombstone son sous-arbre entier,
plutôt que de laisser des dossiers orphelins dont le mode effectif
n'est plus calculable.

### 6.3 Routes

| Route | Rôle |
|---|---|
| `POST /v1/spaces` | créer un espace (premium + web_user lié, comme `publish`) |
| `POST /v1/spaces/invite` | émettre un code d'invitation |
| `POST /v1/spaces/join` | consommer un code, devenir membre |
| `POST /v1/spaces/pull` | `{space_id, since_seq}` -> dossiers et notes changés, tombstones comprises, `next_seq` |
| `POST /v1/spaces/folder` | créer ou modifier un dossier (nom, mode, parent) |
| `POST /v1/spaces/note` | créer ou modifier une note |
| `POST /v1/spaces/note/delete` | tombstoner une note dont on est l'auteur |
| `POST /v1/spaces/folder/move` | déplacer un dossier, refus si cycle |
| `POST /v1/spaces/folder/delete` | tombstoner un dossier et son sous-arbre |
| `POST /v1/spaces/member/remove` | révoquer un membre (propriétaire) |
| `POST /v1/spaces/leave` | quitter, avec option de retrait de ses notes |

`pull` est la seule route de lecture, et la seule qui coûte.

Le gate premium porte sur le PROPRIÉTAIRE de l'espace, jamais sur
l'appelant. `gate::is_premium` teste le compte de l'appareil qui
appelle; l'appliquer tel quel à `join` ou `pull` rendrait la
fonctionnalité morte pour tout invité non payant, ce qui est
exactement l'inverse du produit voulu. Créer un espace exige le
premium; y participer non.

Toute route d'espace répond le même 404 quand l'appelant n'a pas de
droit sur `space_id`: inconnu, jamais rejoint, révoqué, expiré. C'est
la discipline déjà tenue par le plan `shares`, et un 403 sur `pull`
dirait à un membre révoqué que l'espace existe encore.

L'authentification appareil et la limitation de débit globale
(`ratelimit::layer`) s'appliquent inchangées; `pull` reçoit en plus
une pagination par nombre de lignes, avec reprise par `seq`.

### 6.4 Modèle app

Migration V26, colonnes ajoutées aux tables existantes plutôt que
tables parallèles:

```
folders   += space_id TEXT, remote_id TEXT, mode TEXT
notes     += space_id TEXT, remote_id TEXT, author_ref TEXT
spaces      (id, name, owner_ref, joined_at, cursor, last_pull_at)
```

Une note d'espace est une ligne de `notes` ordinaire. Elle passe par
`note_persistence` et `embed::embed_note` comme n'importe quelle note,
donc elle est cherchable et utilisable en chat sans travail
supplémentaire.

C'est le même choix de stockage que `keep_note`, pas le même cycle de
vie: `keep_note` capture une fois et fige, une note d'espace est
réécrite à chaque `pull` tant qu'elle vit. Une note d'espace n'est
donc jamais éditable localement par un non-auteur, sous peine de voir
sa modification écrasée au `pull` suivant.

Les nouvelles colonnes doivent être déclarées dans le catalogue de sync
(`sync/protocol/catalog.rs`), dont les `cols` de `folder` et `note`
sont une liste fixe: une colonne absente de cette liste ne voyage
jamais entre les appareils d'un même compte. `spaces` et
`pending_purge` restent hors catalogue: un curseur et une intention de
purge sont propres à un appareil. `wipe_local_content`
(`note_repo.rs:587`) porte aussi une liste de tables codée en dur, à
compléter, sinon « effacer mes données » laisse les lignes d'espace.

Contrainte reprise de V25: pas de clé étrangère vers `notes`.
L'applicateur de sync fait `INSERT OR REPLACE`, une cascade effacerait
ces lignes à chaque écho d'un pair. Nettoyage explicite dans le chemin
de suppression, comme `delete_note` le fait déjà pour `note_shares` et
`note_provenance`.

### 6.5 Fraîcheur et budget

Un `pull` par espace au premier plan de l'application, un au premier
affichage d'un dossier de l'espace, avec un plancher de 30 secondes
entre deux appels pour le même espace. Un `pull` sans changement
renvoie une liste vide: le coût est un aller-retour, pas un transfert.

Hors ligne, l'espace reste lisible avec ce qui est déjà local; l'écran
affiche l'horodatage du dernier `pull` réussi. Une écriture hors ligne
est refusée plutôt que mise en file: une file d'attente rouvrirait la
question de l'autorité, qui est justement ce que A2 ferme.

### 6.6 Suppression, de bout en bout

```
auteur supprime -> space_notes.deleted_at + seq++
  -> pull des autres appareils
    -> note_persistence::delete_note(local_id)
      -> SQLite: note, audio, provenance, share
      -> pending_purge(note_id) posé
      -> LanceDB delete_note_chunks -> pending_purge levé
```

La purge devient rejouable, et c'est la correction la plus importante
de ce document. Aujourd'hui `delete_note_embeddings` lance un thread
détaché et jette l'erreur (`let _ = store.delete_note_chunks(...)`,
`application/embed/mod.rs:213`): si LanceDB est fermé, occupé ou en
échec, la note est partie de SQLite et son vecteur reste
interrogeable. Une table locale `pending_purge(note_id, kind,
queued_at)` porte l'intention; elle est vidée au démarrage et après
chaque `pull`, jusqu'à succès. Sans elle, la conformité repose sur un
`let _`.

Deuxième trou, hors du chemin `pull`: l'applicateur de sync P2P
supprime la note et ses chunks SQLite mais n'appelle jamais LanceDB
(`sync/protocol/apply/entity.rs`, `DELETE FROM chunks` seulement).
Une note d'espace supprimée sur un appareil et propagée en P2P vers un
autre appareil du même compte y laisse donc son vecteur. Le même
`pending_purge` couvre ce chemin: l'applicateur y pose une ligne au
lieu d'appeler LanceDB depuis le contexte de sync.

Révocation d'un membre: `removed_at` posé, tout `pull` ultérieur
répond 403. Les notes déjà chez lui restent sur son appareil, sauf
s'il a demandé leur retrait.

Départ ou révocation, côté partant: avant de couper l'accès,
l'application propose de conserver ses notes. Conserver recopie ses
notes dans un dossier local hors espace (`space_id` mis à NULL,
`remote_id` effacé): la copie devient une note ordinaire, elle
n'attend plus aucun `pull`.

Retrait de ses contributions: `POST /v1/spaces/leave` avec l'option de
retrait tombstone chaque note dont il est l'auteur, ce qui les fait
disparaître chez tous les autres membres par le chemin ci-dessus.

### 6.7 Ce qui ne change pas

- Le plan P2P `infrastructure/sync/` reste réservé aux appareils d'un
  même compte. Un espace ne passe jamais par lui.
- Le plan `shares` de la proposal 0001 reste tel quel: partager une
  note isolée ne devient pas un espace.
- L'usage solo hors ligne ne dépend d'aucune des routes ci-dessus.

## 7. Inconvénients et risques

**Le contenu d'équipe part en clair sur le serveur.** Autorité serveur
veut dire contenu lisible côté serveur, comme le plan `shares`
aujourd'hui. Le chiffrement de bout en bout est fermé tant que A2
tient. À dire dans l'interface plutôt qu'à sous-entendre.

**Une note d'espace est une note locale.** C'est ce qui donne
l'indexation gratuite, et c'est aussi ce qui la fait voyager en P2P
vers les autres appareils du même compte, et entrer dans la sauvegarde.
Deux endroits à vérifier: le catalogue de sync
(`sync/protocol/catalog.rs`) et `wipe_local_content`, dont la liste de
tables est codée en dur.

**La purge vectorielle est le point de rupture le plus probable.**
`delete_note_embeddings` est asynchrone et LanceDB peut échouer sans que
SQLite le sache. Une note effacée en base mais présente dans l'index
reste interrogeable: c'est exactement le défaut que le brief interdit.
Il faut une purge rejouable, pas un appel qu'on suppose passé.

**Le curseur `seq` suppose un ordre serveur fiable.** Deux écritures
concurrentes dans la même transaction doivent recevoir des numéros
distincts et croissants, sinon un appareil saute un changement. La
génération du `seq` doit se faire sous transaction, dans la même
requête que l'écriture.

**Refuser l'écriture hors ligne est un choix visible.** Un membre dans
le métro ne peut pas déposer de note dans l'espace. C'est assumé:
l'alternative est une file d'attente, donc des conflits, donc l'autorité
partagée que la conception écarte.

**Le quota et la modération changent d'échelle.** `MAX_ACTIVE_SHARES`
compte des codes; un espace est un objet vivant qui grossit. Un espace
sans plafond de notes ni de membres est un trou de coût.

**La suppression forcée des notes d'un membre révoqué est une question
juridique ouverte**, pas seulement technique. Tant qu'elle n'est pas
tranchée, la conception ne l'expose pas: seul le partant peut retirer
ses notes.

## 8. Questions ouvertes

1. Un propriétaire peut-il effacer les contributions d'un membre
   révoqué à son insu? Vérification juridique avant d'ouvrir ce chemin.
2. Un espace a-t-il une expiration comme les partages de la proposal
   0001, ou vit-il tant que le propriétaire paie?
3. Plafonds: membres par espace, notes par espace, octets par note.
4. Une note vocale partagée transporte-t-elle son fichier audio, ou
   seulement sa transcription? Le plan `shares` ne transporte que du
   texte aujourd'hui.
5. La modération de la proposal 0001 (signalement, blocage, file
   admin) s'applique-t-elle à l'échelle de l'espace, ou faut-il un
   signalement par espace?
6. Que devient une note dont l'auteur part sans demander de retrait?
   Proposition par défaut: elle reste, l'auteur est grisé, comme
   l'état `gone` des provenances.

## 9. Recommandation et justification

Retenir A2 + B2 + C1.

A2 parce que le produit tourne déjà sur ce plan: authentification
appareil, gate premium, identité de profil public, tout est là et
éprouvé. A3 est la seule voie vers le chiffrement de bout en bout et
elle bute sur une contrainte plateforme déjà mesurée, pas sur une
préférence.

B2 parce que la règle « une restriction descend » devient vraie par
construction. Toute autre option demande de réécrire les enfants au
moment où le parent change, donc de tomber en incohérence dès qu'un
appareil rate l'écriture.

C1 parce que la fraîcheur et la suppression sont le même problème: un
changement à propager. Un seul curseur les porte tous les deux, et le
plafond de coût du brief tient sans mécanisme séparé.

Le pari est que l'écriture concurrente sur une même note n'existe pas
dans ce produit: chacun écrit ses notes. Si ce pari tombe, c'est
l'axe A qu'il faut rouvrir, pas les autres.

## 10. Plan d'implémentation

Hors périmètre de ce document, donc absents du tableau: l'onboarding
compte, l'interface d'invitation et le QR code (tâches 1 et 3 du
brief).

| ID | Title | Files | Depends on | Effort |
|---|---|---|---|---|
| T01 | Backend migration 18: `spaces`, `space_members`, `space_folders`, `space_notes`, `space_invites`, index par `(space_id, seq)` | `marketplace-flowflow/src/db/migrations.rs` | - | M |
| T02 | Mode effectif + garde d'écriture + plafond de profondeur + refus de cycle, testés seuls | `features/spaces/perm.rs` | T01 | M |
| T03 | Routes create / invite / join / member.remove / leave, gate premium sur le propriétaire, 404 uniforme, code d'invitation à usage unique | `features/spaces/routes.rs`, `src/lib.rs`, `gate.rs` | T02 | M |
| T04 | Routes folder / folder.move / folder.delete / note / note.delete, `seq` attribué sous transaction depuis un compteur porté par `spaces` | `features/spaces/routes.rs`, `repo.rs` | T02 | M |
| T05 | Route `pull` par curseur, tombstones comprises, paginée avec reprise par `seq` | `features/spaces/routes.rs`, `repo.rs` | T04 | M |
| T06 | Quota et plafonds d'espace (membres, notes, taille de note) + limitation de débit sur `pull` | `features/spaces/`, `gate.rs`, `ratelimit.rs` | T03 | S |
| T07 | App: migration V26 (`spaces`, `pending_purge`, colonnes `folders` et `notes`), colonnes déclarées au catalogue de sync, `spaces` et `pending_purge` hors catalogue | `persistence/schema.rs`, `sync/protocol/catalog.rs` | - | M |
| T08 | App: client backend `spaces` | `src/infrastructure/backend/spaces.rs` | T05, T07 | M |
| T09 | App: cas d'usage `application/space.rs` (join, pull, apply delta, write) | `src/application/space.rs` | T08 | L |
| T10 | App: application d'un delta = notes locales réelles, indexation par le pipeline existant | `application/space.rs`, `application/embed/` | T09 | M |
| T11 | App: `pending_purge` rejouable au démarrage et après chaque `pull`; `delete_note_embeddings` cesse d'avaler son erreur | `application/embed/mod.rs`, `application/note_persistence.rs`, `infrastructure/vectordb.rs` | T10 | M |
| T11b | App: le chemin de suppression P2P pose lui aussi une ligne `pending_purge` (aujourd'hui il ne purge que les chunks SQLite) | `sync/protocol/apply/entity.rs` | T11 | S |
| T12 | App: mode de dossier dans l'interface, droit d'écrire visible avant d'écrire | `ui/sidebar/folders.rs`, `ui/notes/row_menu.rs`, `ui/notes/folder_picker.rs` | T09 | M |
| T13 | App: cadence de `pull` (premier plan, ouverture de dossier, plancher 30 s) + horodatage de fraîcheur | `ui/app/watchers.rs`, `application/space.rs` | T09 | S |
| T14 | Sortie et révocation: conserver ses notes en dossier local, option de retrait | `application/space.rs`, `ui/` | T09, T04 | M |
| T15 | `wipe_local_content` couvre `spaces` et `pending_purge`; sauvegarde et restauration transportent les colonnes d'espace | `persistence/note_repo.rs`, `application/backup/` | T10 | S |
| T16 | Protocole de test à deux appareils et deux comptes, jusqu'à zéro note fantôme après retrait | `docs/proposals/0002-.../verification-bundle.md` | tous | M |

```
graph TD
  T01 --> T02 --> T03 --> T06
  T02 --> T04 --> T05 --> T08
  T07 --> T08 --> T09 --> T10 --> T11 --> T11b --> T16
  T09 --> T12 --> T16
  T09 --> T13
  T09 --> T14 --> T16
  T10 --> T15
```

## 11. Conclusions de la revue

Revue adversariale contre le code des deux dépôts. 13 constats, 3
bloquants. Tous appliqués au texte ci-dessus, sauf mention contraire.

| Gravité | Constat | Traitement |
|---|---|---|
| BLOCKER | Le gate premium de `publish` teste l'appareil appelant (`gate.rs`): appliqué à `join` ou `pull`, il tue la fonctionnalité pour tout invité non payant | §6.3: le gate porte sur le propriétaire de l'espace |
| BLOCKER | `delete_note_embeddings` est un thread détaché qui avale son erreur (`embed/mod.rs:213`): la purge promise n'existe pas | §6.6: table `pending_purge` rejouable, T11 réécrite |
| BLOCKER | Le chemin de suppression P2P ne purge que les chunks SQLite, jamais LanceDB (`sync/protocol/apply/entity.rs`) | §6.6 + T11b |
| MAJOR | Un 403 sur `pull` révélerait l'existence d'un espace à un révoqué, contre la discipline 404 du plan `shares` | §6.3: 404 uniforme |
| MAJOR | Aucune route de suppression ni de déplacement de dossier, alors que le document raisonne sur le déplacement de sous-arbre | §6.3: `folder/move`, `folder/delete`; §6.2: refus de cycle |
| MAJOR | Les colonnes ajoutées à `folders` et `notes` ne voyagent pas en P2P sans déclaration dans `catalog.rs` (liste fixe) | §6.4, T07 |
| MAJOR | `wipe_local_content` (`note_repo.rs:587`) a sa liste de tables codée en dur: les lignes d'espace survivraient à « effacer mes données » | §6.4, T15 |
| MAJOR | `seq` attribué hors transaction peut se dupliquer ou sauter avec un pool sqlx multi-connexions | §6.1: compteur porté par `spaces`, `RETURNING` sous transaction |
| MINOR | Profondeur de dossier non bornée, donc coût de résolution non borné | §6.2: plafond de 8 niveaux |
| MINOR | Limitation de débit non traitée pour `spaces` alors que `ratelimit::layer` existe | §6.3, T06 |
| MINOR | Réutilisation d'un code d'invitation non spécifiée | §6.1: usage unique, réintégration par nouveau code |
| NIT | L'analogie avec `keep_note` masque une différence de cycle de vie (capture unique contre réécriture continue) | §6.4 |
| NIT | Le sort des embeddings d'une note conservée en sortant n'était pas dit | §6.6: l'identifiant local ne change pas, rien à repurger |
