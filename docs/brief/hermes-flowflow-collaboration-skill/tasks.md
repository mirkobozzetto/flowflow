---
feature: "Hermes FlowFlow collaboration skill"
slug: hermes-flowflow-collaboration-skill
type: tasks
source_brief: docs/brief/hermes-flowflow-collaboration-skill/brief.md
status: ready
stepsCompleted: [0, 1, 2, 3]
---

# Tâches de collaboration Hermes et FlowFlow

> Do NOT implement. This is the derived task list. Run `ship` to execute.

## Relevant Files

- `docs/brief/hermes-flowflow-collaboration-skill/brief.md` - Source produit.
- `docs/brief/hermes-flowflow-collaboration-skill/tasks.md` - Plan dérivé.
- `../vps-hermes/` - Livraison réutilisable pour Hermes et sa documentation.
- `../marketplace-flowflow/src/features/mcp_spaces/` - Capacités FlowFlow MCP.
- `../marketplace-flowflow/tests/mcp_spaces_test.rs` - Contrat MCP existant.
- `src/ui/sidebar/space_section.rs` - Création et révocation de l'accès Hermes.
- `src/application/i18n/locales/en.ftl` - Parcours utilisateur en anglais.
- `src/application/i18n/locales/fr.ftl` - Parcours utilisateur en français.
- `src/domain/note.rs` - Modèle local des notes et de leurs métadonnées.
- `src/ui/thread/` - Comportement actuel des threads dans FlowFlow.
- `src/application/space/` - Synchronisation d'un espace sur l'iPhone.
- `tests/thread_test.rs` - Contrats existants des threads.
- `tests/space_client_test.rs` - Contrats du client d'espace.
- `tests/space_delta_test.rs` - Application locale des changements distants.

## Tasks

- [ ] 1.0 Cadrer le modèle FlowFlow compris par Hermes
  _(brief: US3, US9)_
  - [ ] 1.1 Décrire espace, dossier, note, thread, auteur et métadonnées.
  - [ ] 1.2 Distinguer les capacités disponibles des capacités manquantes.
  - [ ] 1.3 Définir les limites de lecture, écriture et suppression.
  - [ ] 1.4 Définir le comportement devant une cible ambiguë ou absente.
  - [ ] 1.5 Expliquer la règle d'un seul Hermes actif par espace.
  - [ ] 1.6 Aligner le vocabulaire entre FlowFlow, Hermes et la documentation.

- [ ] 2.0 Créer le parcours réutilisable d'installation et de connexion
  _(brief: US1)_
  - [ ] 2.1 Partir d'un utilisateur sans intégration configurée.
  - [ ] 2.2 Guider la création de l'accès dans l'espace choisi.
  - [ ] 2.3 Expliquer l'affichage unique et la protection du jeton.
  - [ ] 2.4 Installer les connaissances et le parcours FlowFlow dans Hermes.
  - [ ] 2.5 Configurer la connexion sans exposer le secret dans le dialogue.
  - [ ] 2.6 Vérifier la connexion, l'espace et les actions disponibles.
  - [ ] 2.7 Terminer par une première lecture guidée dans l'espace.
  - [ ] 2.8 Documenter la révocation et la reconnexion avec un nouveau jeton.

- [ ] 3.0 Fournir la documentation et le diagnostic pédagogique
  _(brief: US2, US9)_
  - [ ] 3.1 Organiser une documentation consultable après l'installation.
  - [ ] 3.2 Séparer panne du backend, refus d'accès et contenu absent.
  - [ ] 3.3 Expliquer clairement un espace, dossier, note ou thread vide.
  - [ ] 3.4 Décrire les ressources réellement visibles avant de conclure.
  - [ ] 3.5 Fournir une prochaine action pour chaque erreur connue.
  - [ ] 3.6 Masquer les secrets dans chaque diagnostic et confirmation.
  - [ ] 3.7 Permettre l'ajout progressif de capacités et d'erreurs documentées.

- [ ] 4.0 Permettre la découverte et la recherche des notes
  _(brief: US4)_
  - [ ] 4.1 Présenter titre, auteur, date et emplacement dans les résultats.
  - [ ] 4.2 Retrouver une note par un mot de son titre.
  - [ ] 4.3 Retrouver une note par un mot de son contenu.
  - [ ] 4.4 Filtrer les résultats par date ou ancienneté.
  - [ ] 4.5 Présenter plusieurs résultats sans choisir à la place du lecteur.
  - [ ] 4.6 Demander une précision avant une action sur une cible ambiguë.
  - [ ] 4.7 Signaler une recherche vide sans inventer de résultat.
  - [ ] 4.8 Respecter les limites de l'espace dans chaque recherche.

- [ ] 5.0 Permettre la lecture ordonnée des threads
  _(brief: US5)_
  - [ ] 5.1 Identifier les threads disponibles dans un dossier.
  - [ ] 5.2 Présenter le titre et les métadonnées de chaque thread.
  - [ ] 5.3 Lire les notes d'un thread dans leur ordre réel.
  - [ ] 5.4 Distinguer le thread des notes qui le composent.
  - [ ] 5.5 Retrouver un thread à partir de ses titres et contenus.
  - [ ] 5.6 Signaler clairement un thread vide, incomplet ou inaccessible.
  - [ ] 5.7 Empêcher toute lecture hors de l'espace autorisé.

- [ ] 6.0 Permettre l'analyse et la recherche web depuis une note
  _(brief: US6)_
  - [ ] 6.1 Résumer uniquement les notes ou threads demandés.
  - [ ] 6.2 Séparer le contenu FlowFlow des interprétations de Hermes.
  - [ ] 6.3 Permettre à Hermes de proposer des pistes liées au contenu.
  - [ ] 6.4 Lancer une recherche web uniquement lorsque demandée.
  - [ ] 6.5 Séparer les sources web du contenu original de FlowFlow.
  - [ ] 6.6 Rendre les sources consultables depuis la réponse de Hermes.
  - [ ] 6.7 Ne rien écrire dans FlowFlow sans instruction supplémentaire.

- [ ] 7.0 Permettre une participation contrôlée dans l'espace
  _(brief: US7, US9, US10)_
  - [ ] 7.1 Créer un sous-dossier collaboratif dans un parent autorisé.
  - [ ] 7.2 Écrire une note uniquement sur instruction explicite.
  - [ ] 7.3 Mettre à jour uniquement une note créée par Hermes.
  - [ ] 7.4 Exiger une confirmation avant de supprimer sa propre note.
  - [ ] 7.5 Refuser de modifier ou supprimer une note humaine.
  - [ ] 7.6 Garantir qu'un rejeu ne crée pas de doublon.
  - [ ] 7.7 Rendre l'écriture visible une seule fois sur l'iPhone.
  - [ ] 7.8 Confirmer le résultat dans Hermes et Telegram si disponible.
  - [ ] 7.9 Créer une trace FlowFlow seulement sur demande explicite.
  - [ ] 7.10 Signaler une écriture refusée avec la correction attendue.

- [ ] 8.0 Ajouter la personnalisation et les routines explicites
  _(brief: US8)_
  - [ ] 8.1 Recueillir les dossiers, sujets et usages de l'utilisateur.
  - [ ] 8.2 Restituer les préférences avant leur première application.
  - [ ] 8.3 Conserver les préférences séparées du modèle général FlowFlow.
  - [ ] 8.4 Faire évoluer le contexte à la demande de l'utilisateur.
  - [ ] 8.5 Créer une routine seulement après une demande explicite.
  - [ ] 8.6 Définir pour chaque routine cible, fréquence et résultat attendu.
  - [ ] 8.7 Permettre de consulter, modifier, suspendre et arrêter une routine.
  - [ ] 8.8 Garantir qu'aucune surveillance n'est active par défaut.

- [ ] 9.0 Valider les dix scénarios de recette de bout en bout
  _(brief: US1 à US10, Success metrics)_
  - [ ] 9.1 Préparer un espace réel avec dossiers, notes datées et thread.
  - [ ] 9.2 Exécuter le parcours depuis une installation Hermes vierge.
  - [ ] 9.3 Valider installation, connexion et diagnostic.
  - [ ] 9.4 Valider découverte, recherche et choix entre plusieurs résultats.
  - [ ] 9.5 Valider lecture ordonnée, résumé et interprétation d'un thread.
  - [ ] 9.6 Valider une recherche web avec sources séparées.
  - [ ] 9.7 Valider sous-dossier, création, mise à jour et suppression
    autorisée.
  - [ ] 9.8 Exercer les neuf actions MCP avec leur résultat attendu.
  - [ ] 9.9 Prouver l'absence d'accès hors espace et d'écriture implicite.
  - [ ] 9.10 Prouver l'absence de doublon après rejeu.
  - [ ] 9.11 Vérifier la confirmation dans Hermes et Telegram.
  - [ ] 9.12 Vérifier la visibilité unique de la note sur l'iPhone.
  - [ ] 9.13 Vérifier le masquage des secrets dans toutes les sorties.
  - [ ] 9.14 Ajouter à la documentation les erreurs réellement rencontrées.
  - [ ] 9.15 Accepter la livraison seulement après dix scénarios sur dix.
