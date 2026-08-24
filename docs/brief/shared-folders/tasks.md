---
feature: Shared folders (collaborative spaces)
slug: shared-folders
type: tasks
source_brief: docs/brief/shared-folders/brief.md
proposal: docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `ship` (or the
> implementer) to execute.

Les tâches 2, 4, 6 et 7 dépendent des choix figés dans la proposal
0002 (plan `spaces`, mode effectif, curseur `seq`, purge vectorielle).

## Relevant Files

- `src/domain/folder.rs` - mode de dossier, résolution d'héritage
- `src/domain/share.rs` - modèles de partage existants
- `src/application/space.rs` - cas d'usage d'espace (nouveau)
- `src/application/sharing.rs` - partage note/thread existant
- `src/application/note_persistence.rs` - suppression et nettoyage
- `src/application/embed/mod.rs` - indexation et purge d'embeddings
- `src/infrastructure/backend/spaces.rs` - client backend (nouveau)
- `src/infrastructure/persistence/schema.rs` - migration V26
- `src/infrastructure/persistence/folder_repo.rs` - dossiers
- `src/infrastructure/vectordb.rs` - suppression de chunks
- `src/ui/sidebar/folders.rs` - arbre de dossiers et menus
- `src/ui/notes/row_menu.rs` - entrée de partage
- `src/ui/settings/account.rs` - compte et activation
- `marketplace-flowflow/src/db/migrations.rs` - migration 18
- `marketplace-flowflow/src/features/spaces/` - routes et repo (nouveau)
- `marketplace-flowflow/src/lib.rs` - table des routes

## Tasks

- [ ] 1.0 Onboarding compte depuis l'application  _(brief: stories 3, 4)_
  - [ ] 1.1 Un appareil sans compte activé voit une entrée de création
        de compte dans Réglages
  - [ ] 1.2 Le parcours de création se termine sans quitter
        l'application
  - [ ] 1.3 Un parcours d'adhésion interrompu par la création de compte
        reprend là où il s'était arrêté
  - [ ] 1.4 L'état du compte est lisible d'un coup d'oeil (activé, lié,
        appareils)

- [ ] 2.0 Espace et permissions de dossier  _(brief: stories 1, 2, 6)_
  - [ ] 2.1 Un dossier peut devenir un espace partagé depuis son menu
        de ligne
  - [ ] 2.2 Un dossier porte un mode visible, lecture seule ou
        collaboratif
  - [ ] 2.3 Le droit réel d'un dossier est le plus restrictif de sa
        chaîne d'ancêtres
  - [ ] 2.4 Un membre peut créer un sous-dossier lecture seule dans un
        parent collaboratif
  - [ ] 2.5 Le droit d'écrire est visible avant d'écrire
  - [ ] 2.6 La règle est opposable côté serveur, pas seulement dans
        l'interface

- [ ] 3.0 Invitation et adhésion  _(brief: stories 3, 4)_
  - [ ] 3.1 Le propriétaire émet une invitation partageable
  - [ ] 3.2 L'invitation existe comme lien et comme QR code
  - [ ] 3.3 Ouvrir l'invitation sur un appareil équipé mène à l'écran
        « rejoindre cet espace »
  - [ ] 3.4 Ouvrir l'invitation sans FlowFlow explique quoi faire
  - [ ] 3.5 Le nom affiché d'un membre vient de son profil public

- [ ] 4.0 Diffusion vivante des notes  _(brief: story 5)_
  - [ ] 4.1 Une note ajoutée par un membre arrive chez les autres sans
        action manuelle
  - [ ] 4.2 La mise à jour ne transfère que ce qui a changé
  - [ ] 4.3 Au plus un appel par ouverture d'application et par
        ouverture de dossier
  - [ ] 4.4 Hors ligne, l'espace reste lisible et la fraîcheur est
        affichée
  - [ ] 4.5 Une écriture hors ligne est refusée clairement

- [ ] 5.0 Notes reçues traitées comme les siennes  _(brief: story 7)_
  - [ ] 5.1 Une note reçue est indexée par le pipeline existant
  - [ ] 5.2 Elle est trouvable en recherche et utilisable en chat
  - [ ] 5.3 Chaque note affiche son auteur
  - [ ] 5.4 Un membre ne peut modifier que ses propres notes

- [ ] 6.0 Révocation, sortie, récupération  _(brief: stories 8, 9)_
  - [ ] 6.1 Le propriétaire révoque un membre, l'accès se ferme au
        prochain contact serveur
  - [ ] 6.2 En sortant ou en étant révoqué, la personne se voit
        proposer de conserver ses notes
  - [ ] 6.3 Conserver recopie ses notes dans un dossier local hors
        espace
  - [ ] 6.4 La personne peut demander le retrait de ses notes de
        l'espace
  - [ ] 6.5 Supprimer l'espace le fait disparaître chez tous les
        membres

- [ ] 7.0 Effacement partout, index compris  _(brief: story 10)_
  - [ ] 7.1 Une suppression voyage jusqu'à chaque appareil membre
  - [ ] 7.2 La copie locale et ses embeddings partent ensemble
  - [ ] 7.3 Une purge vectorielle échouée est rejouée, jamais perdue
  - [ ] 7.4 Une note supprimée ne ressort ni en recherche ni en chat
  - [ ] 7.5 Les notes d'espace se comportent correctement dans la
        sauvegarde, la restauration et l'effacement des données locales

- [ ] 8.0 Preuve à deux appareils  _(brief: Success metrics)_
  - [ ] 8.1 Un espace, deux comptes, deux appareils
  - [ ] 8.2 20 notes échangées dans les deux sens
  - [ ] 8.3 Une note visible chez l'autre en moins de 60 secondes
  - [ ] 8.4 Zéro note fantôme après retrait et après révocation
  - [ ] 8.5 L'usage solo hors ligne est intact
