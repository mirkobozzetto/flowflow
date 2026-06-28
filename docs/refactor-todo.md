# Refactor backlog - FlowFlow

Liste des refactos SRP restants, classés par rapport payoff / risque. Chaque
tâche tient en deux lignes : une description autonome, puis la commande à
lancer telle quelle.

Règle du symptôme : faire en priorité ce que tu vas toucher de toute façon ou
ce qui fait mal à maintenir, pas tout d'un coup. C'est une carte, pas une
urgence. Chaque refacto se fait pas à pas, sur une base verte (compile + check
vert) entre chaque coupe.

Le skill `srp-refactor` gère ces tâches : il inventorie d'abord les
responsabilités (gate obligatoire), propose le découpage, exécute une coupe à
la fois, vérifie vert, puis propose la suite.

## 1. Sortir le fixture JSON de connector_module (win trivial, zéro risque)

Le fichier `src/application/connector_module.rs` embarque un gros fixture JSON
inline qui gonfle le module ; le déplacer dans un fichier de données ou de test
et le charger, le module retombe à environ 200 lignes.
`/ship sortir le fixture inline de src/application/connector_module.rs vers un fichier de données ou tests/fixtures/, charger depuis le fichier`

## 2. Découper le composant detail.rs (le prochain mod.rs)

`src/ui/notes/detail.rs` est un seul composant de 669 lignes qui mélange le
rendu, l'état (14 signaux), des appels base de données inline, l'import audio
et fichier, et la transcription ; extraire des hooks (use_note_editor,
use_audio_import, use_file_import) et pousser la logique métier dans
`application/`.
`/srp-refactor src/ui/notes/detail.rs`

## 3. Découper rag.rs (recherche / reranking / temporel / orchestration)

`src/application/rag.rs` (628 lignes) a une fonction `query` de 173 lignes qui
orchestre tout, plus des clusters distincts : recherche vectorielle, reranking,
détection temporelle, fusion des résultats ; les séparer en sous-modules.
`/srp-refactor src/application/rag.rs`

## 4. Dégonfler apply_row dans apply.rs et sortir les tests

`src/infrastructure/sync/protocol/apply.rs` (1004 lignes) contient une
god-function `apply_row` de 239 lignes à découper en sous-handlers, et des
tests inline à déplacer dans `tests/`.
`/srp-refactor src/infrastructure/sync/protocol/apply.rs`

## 5. Découper le composant connections.rs

`src/ui/settings/connections.rs` est un composant de 426 lignes qui pilote la
logique connecteur (connect, disconnect, binding de feuilles) inline ; extraire
un hook use_connector et des sous-composants, déléguer à `application/`.
`/srp-refactor src/ui/settings/connections.rs`

## 6. Découper backup.rs (le plus gros, en dernier)

`src/application/backup.rs` (2089 lignes) mélange export, import, validation,
swap et état de restauration, plus des tests inline ; le splitter en
`backup/{export,import,validate,state}.rs` et sortir les tests dans `tests/`.
Le plus couplé, à faire en dernier, une coupe à la fois sur base verte.
`/srp-refactor src/application/backup.rs`

## Transverse : tests inline dans src/

Plusieurs fichiers (backup.rs, apply.rs) portent des tests inline, ce qui viole
la règle "tests dans tests/, jamais inline dans src/" ; les déplacer au fur et
à mesure des refactos ci-dessus, ou en une passe d'hygiène dédiée.
`/ship déplacer les tests inline de src/ (backup.rs, apply.rs) vers tests/<module>.rs, conserver les accès pub minimaux nécessaires`

## Laissés volontairement de côté (cohésifs, pas de symptôme)

- `src/domain/governance.rs` (635) : domaine pur et cohésif (schéma +
  validation + gate). Ne pas splitter sans raison de changement.
- `src/infrastructure/persistence/note_repo.rs` (488) : persistance note
  cohésive ; splitter par sous-entité (audio, contenu) seulement si ça grossit.
