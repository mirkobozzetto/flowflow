---
feature: "Hermes FlowFlow collaboration skill"
slug: hermes-flowflow-collaboration-skill
type: brief
status: ready
next_action: "Installer et valider la collaboration FlowFlow dans Hermes."
resume_cmd: "/ship docs/brief/hermes-flowflow-collaboration-skill"
stepsCompleted: [0, 1, 2]
---

# Collaboration Hermes et FlowFlow

## En bref

Pour toute personne qui utilise FlowFlow avec Hermes Agent.
Hermes doit comprendre les espaces, notes, threads et permissions sans deviner.
Un parcours guidé l'installe, le connecte, le personnalise et prouve ses
actions.

## Problème

La connexion MCP donne à Hermes des outils, mais pas le modèle mental de
FlowFlow ni un parcours d'utilisation fiable. L'utilisateur doit encore
expliquer manuellement ce qu'est un espace, où chercher, comment interpréter
un thread et comment réagir quand une ressource manque.

Cette lacune rend les capacités existantes difficiles à découvrir et à
utiliser. Elle empêche aussi de distinguer une absence de contenu, un manque
de permission, une erreur du backend et une instruction ambiguë.

Le besoin est immédiat : la connexion entre Hermes et FlowFlow existe déjà.
Il faut désormais la rendre compréhensible, réutilisable et vérifiable.

## Objectifs

- Permettre à tout utilisateur FlowFlow et Hermes d'installer l'intégration.
- Expliquer à Hermes le modèle mental de FlowFlow et ses limites.
- Guider l'utilisateur de la création de l'accès jusqu'à la première lecture.
- Rendre les erreurs de connexion et d'autorisation compréhensibles.
- Permettre de retrouver des notes par titre, contenu, date et ancienneté.
- Permettre de lire un thread et l'enchaînement de ses notes.
- Permettre à Hermes de résumer, analyser et prolonger une note.
- Permettre une recherche web à partir d'un contenu FlowFlow demandé.
- Permettre la création de sous-dossiers collaboratifs et de notes.
- Permettre à Hermes de gérer ses propres notes sur instruction explicite.
- Prouver les neuf actions MCP actuellement exposées.
- Personnaliser les usages sans modifier la connaissance générale de FlowFlow.
- Prouver chaque capacité par une recette visible depuis Hermes et l'iPhone.

## Out-of-scope

- Conversation directe avec Hermes depuis le chat FlowFlow.
- Surveillance automatique d'un dossier activée par défaut.
- Création automatique d'une trace de résultat dans FlowFlow.
- Accusés de lecture ou indicateurs « vu par » dans l'application.
- Suppression par Hermes d'une note écrite par un humain.
- Accès à un espace différent de celui associé au jeton.
- Écriture à la racine ou dans un dossier non collaboratif.
- Traitement des pièces jointes dans cette première livraison.
- Choix d'architecture, de format de skill ou de protocole interne.

Les accusés de lecture et le chat direct avec Hermes devront être suivis dans
deux issues produit distinctes.

## User stories

### US1 - Installation guidée

En tant qu'utilisateur FlowFlow et Hermes, je veux un parcours guidé qui
m'explique comment créer et installer l'accès, afin d'obtenir une connexion
fonctionnelle sans connaître le MCP.

### US2 - Diagnostic pédagogique

En tant qu'utilisateur, je veux que Hermes distingue une connexion valide,
un espace vide, une ressource absente et un accès refusé, afin de savoir quoi
corriger sans exposer mon jeton.

### US3 - Compréhension de FlowFlow

En tant qu'utilisateur, je veux que Hermes comprenne les espaces, dossiers,
notes, threads, auteurs, dates et permissions, afin que ses réponses utilisent
le bon vocabulaire et les bonnes limites.

### US4 - Découverte et recherche

En tant qu'utilisateur, je veux demander à Hermes de retrouver une note par
mot, titre, date ou ancienneté, afin de récupérer un contenu sans connaître son
identifiant.

### US5 - Lecture des threads

En tant qu'utilisateur, je veux que Hermes lise un thread dans son ordre et
comprenne les notes qui s'y enchaînent, afin qu'il puisse restituer le fil de
la réflexion.

### US6 - Analyse et prolongement

En tant qu'utilisateur, je veux demander à Hermes de résumer une note, dire ce
qu'elle lui évoque ou effectuer une recherche web liée, afin de poursuivre le
travail depuis son contenu.

### US7 - Participation à l'espace

En tant qu'utilisateur, je veux demander à Hermes de créer un sous-dossier
collaboratif et d'y gérer ses propres notes, afin que son travail soit visible
dans FlowFlow et synchronisé sur mon iPhone.

### US8 - Personnalisation évolutive

En tant qu'utilisateur, je veux définir puis modifier les dossiers, sujets et
routines qui m'intéressent, afin que Hermes adapte son comportement sans
surveillance automatique non demandée.

### US9 - Instruction ambiguë

En tant qu'utilisateur, je veux que Hermes décrive ce qu'il voit et demande
une précision lorsque la cible est ambiguë, afin qu'il ne choisisse pas une
note ou un dossier arbitrairement.

### US10 - Confirmation dans le canal actif

En tant qu'utilisateur, je veux recevoir le résultat dans Hermes et sur
Telegram lorsque ce canal est disponible, afin de savoir que la demande a été
traitée sans polluer FlowFlow.

## Acceptance criteria

### US1 - Installation guidée

- [ ] Le parcours part d'un utilisateur sans intégration configurée.
- [ ] Il explique où créer l'accès dans FlowFlow.
- [ ] Il explique que le jeton n'est affiché qu'une fois.
- [ ] Il empêche de publier le jeton dans une conversation.
- [ ] Il mène jusqu'à une connexion Hermes vérifiée.
- [ ] La documentation reste consultable après l'installation.

### US2 - Diagnostic pédagogique

- [ ] Hermes vérifie la connexion avant de conclure qu'un contenu est absent.
- [ ] Une erreur du backend est distinguée d'un manque de permission.
- [ ] Un espace vide est décrit comme vide, sans être présenté comme une panne.
- [ ] Chaque échec indique la prochaine action utile.
- [ ] Aucun secret complet n'apparaît dans le diagnostic.

### US3 - Compréhension de FlowFlow

- [ ] Hermes explique correctement espace, dossier, note et thread.
- [ ] Hermes distingue les contenus humains de ses propres contenus.
- [ ] Hermes connaît les limites de lecture, d'écriture et de suppression.
- [ ] Hermes ne présente jamais un autre espace comme accessible.

### US4 - Découverte et recherche

- [ ] Une note peut être retrouvée par un mot de son titre.
- [ ] Une note peut être retrouvée par un mot de son contenu.
- [ ] Une note peut être filtrée par date ou ancienneté.
- [ ] Plusieurs résultats sont présentés avec assez de contexte pour choisir.
- [ ] Aucun résultat plausible n'est inventé quand la recherche est vide.

### US5 - Lecture des threads

- [ ] Hermes peut identifier les threads d'un dossier.
- [ ] Il restitue les notes d'un thread dans leur ordre.
- [ ] Il distingue le titre du thread des titres de ses notes.
- [ ] Il signale clairement un thread vide ou incomplet.

### US6 - Analyse et prolongement

- [ ] Hermes peut résumer une note explicitement demandée.
- [ ] Il distingue le contenu FlowFlow de ses propres interprétations.
- [ ] Il peut proposer des pistes liées au contenu.
- [ ] Il peut effectuer une recherche web lorsque l'utilisateur la demande.
- [ ] Les sources web sont séparées du contenu original de la note.

### US7 - Participation à l'espace

- [ ] Hermes crée un sous-dossier uniquement dans un emplacement autorisé.
- [ ] Le sous-dossier créé est collaboratif.
- [ ] Hermes écrit une note uniquement sur instruction explicite.
- [ ] Hermes peut mettre à jour sa propre note sur instruction explicite.
- [ ] Une suppression de sa propre note exige une confirmation explicite.
- [ ] La note apparaît une seule fois dans FlowFlow sur l'iPhone.
- [ ] Hermes ne modifie et ne supprime aucune note écrite par un humain.
- [ ] Un rejeu de la même demande ne crée pas de doublon.

### US8 - Personnalisation évolutive

- [ ] L'utilisateur peut décrire les dossiers et sujets qui l'intéressent.
- [ ] Hermes peut restituer ces préférences avant de les appliquer.
- [ ] L'utilisateur peut modifier ou arrêter une routine existante.
- [ ] Aucune routine n'est active sans demande explicite.
- [ ] La connaissance générale de FlowFlow reste indépendante des préférences.

### US9 - Instruction ambiguë

- [ ] Hermes peut lire les choix disponibles avant de demander une précision.
- [ ] Il décrit les dossiers ou notes réellement visibles.
- [ ] Il demande une précision avant toute écriture ambiguë.
- [ ] Il ne choisit jamais silencieusement une cible incertaine.

### US10 - Confirmation dans le canal actif

- [ ] Hermes confirme le résultat dans la conversation active.
- [ ] Une conversation Telegram reçoit aussi la confirmation si elle est active.
- [ ] Aucun contenu supplémentaire n'est écrit dans FlowFlow par défaut.
- [ ] Une note de résultat est créée seulement sur demande explicite.

## Success metrics

### Métrique principale

Baseline : 0 scénario complet validé sur 10 aujourd'hui.

Cible : 10 scénarios sur 10 réussis lors de la recette finale : installation,
diagnostic, découverte, recherche, thread, résumé, recherche web,
sous-dossier, écriture et visibilité sur l'iPhone.

Fenêtre : une exécution complète de la recette avant acceptation de la
livraison.

### Garde-fous

- 0 accès observé à un espace non autorisé.
- 0 écriture sans instruction explicite.
- 0 suppression d'un contenu humain.
- 0 doublon lors du rejeu de l'écriture de recette.
- 9 actions MCP sur 9 sont exercées avec leur résultat attendu.
- 100 % des erreurs de recette indiquent une prochaine action.
- 100 % des secrets restent masqués dans les sorties et confirmations.

## Contraintes et hypothèses

- L'intégration doit être réutilisable par tout utilisateur FlowFlow et Hermes.
- Un seul Hermes actif est présenté par espace.
- L'utilisateur crée et révoque l'accès depuis FlowFlow.
- Hermes reçoit uniquement l'accès associé à l'espace choisi.
- Hermes peut lire, écrire ses notes et créer des dossiers collaboratifs.
- Hermes ne peut pas supprimer ou modifier une note humaine.
- Une instruction vient de Hermes, Telegram ou d'un opérateur autorisé.
- FlowFlow n'est pas utilisé comme file de commandes implicite.
- Une routine récurrente est optionnelle et explicitement configurée.
- La confirmation se fait dans Hermes et dans Telegram si disponible.
- Une écriture FlowFlow de résultat exige une demande explicite.
- La documentation doit évoluer avec les capacités et erreurs connues.
- La livraison doit inclure une preuve réelle sur un iPhone physique.

## Questions ouvertes

- Quel vocabulaire visuel représentera plus tard les accusés de lecture ?
- Comment la future conversation directe avec Hermes apparaîtra-t-elle dans le
  chat FlowFlow ?
- Les pièces jointes devront-elles rejoindre la recherche dans un brief futur ?
