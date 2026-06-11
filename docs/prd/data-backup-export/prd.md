---
feature: Backup, export & restore des données FlowFlow
slug: data-backup-export
type: prd
status: ready
built: no
stepsCompleted: [0, 1, 2, 3, 4]
---

> NOTE (2026-06-11): spec finalisee (status: ready) mais NON implementee - zero
> code dans `src/`. "ready" = pret a construire, pas construit. La RFC 0001 (revision 2,
> post-sync RFC 0004) porte le design technique a jour et ETEND le scope au desktop
> macOS (decide avec Mirko le 2026-06-11). Implementer via
> `/ship docs/rfcs/0001-data-backup-export/RFC.md` une fois la RFC Accepted.

# PRD: Backup, export & restore des données FlowFlow

## Problem statement

FlowFlow n'a aucun mécanisme de sauvegarde interne. Toutes les données utilisateur
(notes, dossiers, tags, conversations, attachments, enregistrements audio, vecteurs)
vivent uniquement dans le conteneur de l'app sur l'iPhone.

Deux douleurs aujourd'hui:

- **Risque de perte.** Une réinstallation, une perte ou casse du téléphone, ou un
  changement de signature (dev vers TestFlight vers App Store) peut effacer toutes
  les données sans recours.
- **Pas de portabilité.** Les données sont piégées dans l'app. Impossible de les
  sortir pour les archiver, les transférer sur un autre appareil, ou les garder
  hors de l'app.

Le seul contournement actuel (Xcode "Download Container") ne fonctionne que pour une
app signée en développement. Il est inutilisable par un utilisateur final qui aura
installé FlowFlow depuis l'App Store. Pour une app dont la valeur est la mémoire
personnelle de l'utilisateur, l'absence de backup est un risque produit majeur.

## Goals

- Permettre à l'utilisateur d'**exporter** la totalité de ses données dans une archive
  unique, depuis l'app, sans Xcode ni câble.
- Permettre de **partager** cette archive vers l'extérieur (Mac, cloud perso, autre
  appareil) via le partage natif iOS.
- Permettre de **restaurer** une archive dans l'app pour récupérer ou migrer ses données.
- Garantir un aller-retour fidèle: ce qui est exporté revient identique à l'import.
- Rester 100% local et hors-ligne, sans dépendance à un service cloud tiers.
- Ne jamais exposer de secret: les clés API ne quittent jamais l'appareil dans l'archive.

## Non-goals / Out-of-scope

- Pas de synchronisation automatique ni de backup cloud géré (iCloud sync, backend maison).
- Pas de sauvegarde planifiée/automatique en arrière-plan (export = action manuelle).
- Pas de merge ni de résolution de conflits: l'import remplace, il ne fusionne pas.
- Pas d'export sélectif (note par note, dossier par dossier) dans cette version.
- Pas d'export des clés API dans l'archive (elles sont volontairement exclues).
- Pas de format d'échange interopérable avec d'autres apps (l'archive est propre à FlowFlow).
- Pas de chiffrement de l'archive par mot de passe dans cette version.

## User stories

1. **Export complet.** En tant qu'utilisateur, je veux exporter toutes mes données
   dans une archive depuis les réglages, afin de ne jamais risquer de les perdre.
2. **Partage de l'archive.** En tant qu'utilisateur, je veux envoyer l'archive vers
   mon Mac ou mon cloud via le partage iOS, afin de la conserver en lieu sûr.
3. **Restauration.** En tant qu'utilisateur, je veux importer une archive dans l'app,
   afin de récupérer mes données après une réinstallation ou sur un nouveau téléphone.
4. **Sécurité des secrets.** En tant qu'utilisateur, je veux que mes clés API ne soient
   jamais incluses dans l'archive, afin de pouvoir la partager sans fuite.
5. **Sûreté de l'import.** En tant qu'utilisateur, je veux qu'un import raté n'abîme
   jamais mes données actuelles, afin de tester une restauration sans crainte.

## Acceptance criteria

**Story 1 (export complet)**
- Given des notes, dossiers, tags, conversations, attachments, audio et vecteurs présents,
  When je lance l'export depuis les réglages,
  Then une archive unique est produite contenant SQLite, le vector store et les fichiers audio.
- Given l'export terminé, When j'inspecte l'archive, Then aucune clé API n'y figure.

**Story 2 (partage)**
- Given une archive générée, When je choisis de la partager,
  Then la feuille de partage native iOS s'ouvre (AirDrop, Fichiers, mail, cloud).

**Story 3 (restauration)**
- Given une archive FlowFlow valide, When je l'importe,
  Then les données de l'app sont remplacées par celles de l'archive (replace total),
  And après import les notes, audio, tags et la recherche sémantique fonctionnent comme à l'export.
- Given un nouvel appareil/app vierge, When j'importe l'archive, Then mes données réapparaissent à l'identique.

**Story 4 (sécurité des secrets)**
- Given des clés API enregistrées, When j'exporte puis réimporte,
  Then les clés ne sont pas restaurées et l'app demande de les re-saisir.

**Story 5 (sûreté de l'import)**
- Given une archive corrompue ou de version incompatible, When je tente l'import,
  Then l'import est refusé avec un message clair, And les données actuelles restent intactes.
- Given un import, Then la validation se fait avant toute écriture (atomic): aucun état partiel.

## Success metrics

- 100% des données exportables sont restaurées à l'identique sur un aller-retour
  export puis import (0 perte de note/audio/tag/conversation).
- 0 clé API présente dans une archive exportée (vérifiable par inspection).
- 0 cas de corruption des données existantes lors d'un import qui échoue.
- Un export complet d'une base réaliste se déclenche et aboutit en moins de 30 s
  pour un volume cible (à confirmer: par ex. 500 notes + 100 audios).
- Restauration réussie sur un appareil vierge en moins de 3 actions utilisateur
  (importer le fichier, confirmer le replace, attendre la fin).

## Constraints & assumptions

- iOS + desktop macOS (scope étendu le 2026-06-11, l'app desktop n'existait pas à la
  rédaction initiale), 100% Rust/Dioxus, hors-ligne, aucun backend ni cloud tiers.
- L'archive transite par le partage natif iOS et le picker de fichiers existant.
- Cible: utilisateurs finaux App Store (robustesse et UX sans outil dev requis).
- Toutes les données vivent sous le dossier Documents de l'app (préservé par iOS sur update).
- L'import est un **replace total**: il écrase l'état courant par l'archive.
- Les clés API sont exclues de l'archive par décision de sécurité.
- Livraison prévue après la v1.0 App Store (priorité non urgente, mais qualité prod).

## Open questions

- Faut-il confirmer le volume cible exact pour la métrique de durée d'export (500 notes / 100 audios ?).
- Faut-il une confirmation explicite "ceci va écraser vos données actuelles" avant un import replace ? (probablement oui, à valider en UX).
- Stratégie de compatibilité ascendante: comment gérer une archive d'une future version de schéma plus récente que l'app courante (refus simple vs message de mise à jour) ?
- Faut-il proposer plus tard une option de chiffrement par mot de passe pour autoriser l'inclusion des clés (hors scope ici, mais à noter) ?
