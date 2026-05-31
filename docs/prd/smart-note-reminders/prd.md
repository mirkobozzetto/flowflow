---
feature: Smart note-driven reminders
slug: smart-note-reminders
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
---

# PRD — Smart note-driven reminders

## Problem statement

Quand l'utilisateur écrit (ou dicte) une intention datée dans une note ("rappelle-moi
d'appeler Paul demain 15h", "faire les courses samedi", "renouveler l'abonnement le 1er"),
cette intention reste **muette**. La note la stocke comme du texte mort : aucune
notification, aucun suivi. FlowFlow fermé, rien ne rappelle rien.

Aujourd'hui l'utilisateur doit soit **recopier à la main** l'intention dans Rappels ou
Calendrier iOS, soit l'**oublier**. La double saisie casse le flow ("je note vite et je
passe à autre chose"), et l'oubli annule la valeur même de la prise de note.

**Pourquoi maintenant :** les couches notes / transcription / RAG sont mûres. Il manque la
couche **action** : transformer ce que l'utilisateur a écrit en quelque chose que le
système fait pour lui. C'est le premier pas vers des notes "agentiques".

## Goals

- G1 — Une intention temporelle écrite dans une note devient un **rappel iOS réel** qui se
  déclenche au bon moment, **même app fermée**.
- G2 — Zéro double saisie : l'utilisateur ne ressaisit jamais la date/heure ailleurs.
- G3 — L'utilisateur garde le **contrôle** : l'agent propose, l'utilisateur confirme. Jamais
  d'écriture silencieuse.
- G4 — Le rappel reste **éditable et fiable** dans l'écosystème iOS natif (Rappels.app),
  survit au reboot, snoozable.
- G5 — Cohérence du cycle de vie : supprimer la note retire le rappel associé.

## Non-goals / Out-of-scope

- Pas d'Android (iOS only, comme tout FlowFlow).
- Pas de scheduling côté serveur, pas de daemon background maison (impossible sur iOS :
  app suspendue = 0 CPU).
- Pas de sync cross-device au-delà de ce que Rappels/Calendrier iCloud donne gratuitement.
- Pas de moteur de langage naturel maison : l'extraction d'intention s'appuie sur le LLM
  déjà intégré.
- Pas de gestion de tâches complète (sous-tâches, priorités avancées, projets) : on crée un
  rappel, pas un gestionnaire de projet.
- V1 : pas de modification d'un rappel existant depuis la note après création (create +
  delete seulement ; l'édition fine se fait dans Rappels.app).

## User stories

- **US1 — Détection d'intention**
  En tant qu'utilisateur, je veux que FlowFlow repère automatiquement quand une note
  contient une intention datée, pour ne pas avoir à signaler moi-même qu'il y a un rappel à
  créer.

- **US2 — Confirmation avant création**
  En tant qu'utilisateur, je veux valider (ou ignorer) le rappel proposé avant qu'il soit
  créé, pour rester maître de ce qui atterrit dans mes Rappels.

- **US3 — Rappel système fiable**
  En tant qu'utilisateur, je veux que le rappel confirmé se déclenche via iOS au moment
  prévu même si FlowFlow est fermé, pour pouvoir compter dessus.

- **US4 — Récurrence**
  En tant qu'utilisateur, je veux qu'une intention répétée ("tous les lundis", "chaque 1er
  du mois") crée un rappel récurrent, pour ne pas le recréer à chaque fois.

- **US5 — Permission gracieuse**
  En tant qu'utilisateur, je veux comprendre pourquoi l'app demande l'accès aux Rappels, et
  garder un repli utile si je refuse, pour ne pas me sentir piégé.

- **US6 — Cycle de vie propre**
  En tant qu'utilisateur, je veux que supprimer une note retire son rappel, pour ne pas
  garder des rappels orphelins.

## Acceptance criteria

**US1 — Détection**
- Given une note contenant "appeler Paul demain 15h", When la note est sauvegardée/éditée,
  Then un indicateur "rappel détecté" apparaît avec l'action + la date/heure résolues.
- Given une note sans aucune intention temporelle, When elle est sauvegardée, Then aucun
  indicateur n'apparaît (pas de faux positif intrusif).
- Given une date relative ("demain", "samedi", "dans 2 jours"), When l'intention est
  extraite, Then elle est résolue en date absolue à partir de la **date courante**.

**US2 — Confirmation**
- Given un rappel détecté, When l'utilisateur ouvre l'indicateur, Then il voit titre + date
  + heure (+ récurrence si présente) et deux actions : **Confirmer** / **Ignorer**.
- Given l'utilisateur n'a rien confirmé, When il quitte la note, Then aucun rappel n'est
  créé (création = jamais silencieuse).

**US3 — Rappel système**
- Given un rappel confirmé, When l'échéance arrive et FlowFlow est fermé, Then iOS notifie
  l'utilisateur à l'heure prévue.
- Given un rappel confirmé, When l'utilisateur ouvre Rappels.app, Then le rappel y est
  visible, éditable et snoozable.

**US4 — Récurrence**
- Given "tous les lundis à 9h", When confirmé, Then un rappel récurrent hebdomadaire est
  créé et se redéclenche chaque semaine.

**US5 — Permission**
- Given la première création de rappel, When l'app demande l'accès, Then un texte clair
  explique l'usage avant le pop-up système.
- Given l'utilisateur refuse l'accès aux Rappels, When il confirme quand même un rappel,
  Then un repli notification locale est proposé (sans bloquer la feature).

**US6 — Cycle de vie**
- Given une note avec un rappel associé, When la note est supprimée, Then le rappel associé
  est retiré.
- Given une intention déjà confirmée dans une note, When la même note est ré-éditée, Then
  aucun rappel en double n'est créé pour la même intention.

**Exceptions / garde-fous**
- Date détectée déjà passée → avertir, ne pas créer en silence.
- Échec de création (système indisponible) → message d'erreur clair + possibilité de
  réessayer ; rien n'est perdu.
- Intention ambiguë (date sans heure) → appliquer un défaut explicite et affiché
  (ex. matin/soir), modifiable avant confirmation.

## Success metrics

- M1 — **Taux de capture** : ≥ 80 % des notes contenant une intention datée affichent un
  indicateur "rappel détecté" correct (action + date justes) sur un set de test.
- M2 — **Faux positifs** : ≤ 5 % des notes sans intention temporelle déclenchent un
  indicateur.
- M3 — **Conversion** : ≥ 60 % des rappels détectés sont confirmés par l'utilisateur (signal
  que la détection est pertinente).
- M4 — **Fiabilité de déclenchement** : 100 % des rappels confirmés se déclenchent à l'heure
  via iOS app fermée (vérifié on-device).
- M5 — **Zéro orphelin** : 0 rappel restant après suppression de sa note.
- M6 — **Zéro doublon** : 0 rappel dupliqué après ré-édition d'une note déjà traitée.

## Constraints & assumptions

- **Plateforme** : iOS only, 100 % Rust. Aucune exécution en background hors mécanisme
  délégué au système d'exploitation.
- **Contrôle utilisateur** : conformité App Store et confiance → toute écriture dans
  Rappels/Calendrier est précédée d'une confirmation explicite ; jamais d'action silencieuse.
- **Consentement IA** : l'extraction d'intention passe par le LLM, déjà derrière le
  consentement IA existant.
- **Permission système** : créer un rappel exige une autorisation iOS ; l'app doit dégrader
  proprement si refusée.
- **Gate de faisabilité** : avant toute logique produit, valider que la brique iOS
  (Calendrier/Rappels en Rust) compile et se lie réellement sur les cibles iOS du projet —
  prérequis bloquant (règle de validation Track-F du projet).
- **Hypothèse** : la qualité d'extraction du LLM sur les dates FR/EN est suffisante avec la
  date courante injectée ; à mesurer (M1).

## Open questions

- Q1 — Déclencheur de détection : à la sauvegarde de chaque note, ou à la demande (bouton
  "détecter") ? (défaut proposé : à la sauvegarde, throttlé).
- Q2 — Rappels (Reminders) vs événement Calendrier comme cible primaire : Rappels par défaut
  (mieux adapté à "rappelle-moi"), Calendrier en option future.
- Q3 — Granularité du repli notification locale : simple notif unique, ou file gérée ?
- Q4 — Plusieurs intentions dans une seule note : toutes proposées d'un coup, ou une par
  une ?
- Q5 — Langues : FR + EN au lancement (aligné i18n existant) ; autres plus tard ?
