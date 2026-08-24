---
type: tasks
source_brief: brief.md
slug: spaces-test-setup
created: 2026-08-24
---

# Tâches : pouvoir tester les espaces partagés à deux comptes

> Do NOT implement. C'est la liste de travail, pas l'exécution.
> Les tâches marquées **(Mirko)** ne peuvent pas être faites par l'agent.

## Relevant Files

- `docs/proposals/0002-collaborative-shared-folders/verification-bundle-T07-T16.md` — le protocole à jouer
- `~/Library/Application Support/FlowFlow/` — les données de l'app Mac
- `marketplace-flowflow` — backend déployé sur Dokploy
- outils admin marketplace — comptes, appareils, entitlements

## Tasks

- [x] 1.0 Mettre le backend en production _(brief: critère 1)_
  - [x] 1.1 ~~Cliquer Deploy sur Dokploy~~ déjà déployé, constaté et non supposé
  - [x] 1.4 Les onze routes `/v1/spaces/*` répondent 401 en production, une route inventée répond 404 : elles sont montées
  - [ ] 1.5 Brancher l'auto-deploy : activer le webhook côté Dokploy, coller son URL dans les webhooks GitHub du dépôt, pour ne plus jamais déployer à la main

- [ ] 2.0 Créer le second compte web _(brief: critère 2)_
  - [ ] 2.1 **(Mirko)** Choisir une seconde adresse, un alias de la principale suffit
  - [ ] 2.2 **(Mirko)** S'inscrire sur `account.flowflow.be` avec cette adresse et un nouveau passkey
  - [ ] 2.3 Confirmer que deux utilisateurs web distincts apparaissent côté admin
  - [ ] 2.4 Vérifier que ce second compte n'a AUCUN entitlement premium

- [ ] 3.0 Un second appareil, sans rien détruire _(brief: critère 3)_
  - [x] 3.1 ~~Effacer la base du Mac~~ inutile : `FLOWFLOW_DATA_DIR` est la couture prévue pour une seconde instance
  - [x] 3.2 `scripts/tester-app.sh` + cible `make tester` : seconde instance sur `~/FlowFlowTester`
  - [x] 3.3 Instance lancée, identité d'appareil neuve, zéro note, les 69 notes du vrai Mac intactes
  - [x] 3.4 Enregistrée côté backend sur un compte DIFFÉRENT de l'iPhone, et non premium
  - [ ] 3.5 **(Mirko)** Depuis la page compte de cette instance, générer le lien et le redeemer avec le SECOND compte web

- [ ] 4.0 Répartir les rôles _(brief: critère 4)_
  - [ ] 4.1 Confirmer que le compte de l'iPhone est premium, actif, sans expiration
  - [ ] 4.2 Confirmer que le compte du Mac est gratuit et le RESTE, c'est ce qu'on prouve
  - [ ] 4.3 Installer la dernière build sur les deux appareils, `make all` et `make desktop-app`

- [ ] 5.0 Jouer le protocole _(brief: critères 5, 6)_
  - [ ] 5.1 Section 3, migration V26 : appliquée une seule fois, contenu existant intact
  - [ ] 5.2 Section 4.1, créer, inviter, rejoindre : le Mac reçoit l'arborescence sans devenir premium
  - [ ] 5.3 Section 4.2, une note d'un membre atteint l'autre sans republication
  - [ ] 5.4 Section 4.3, la note reçue est cherchable et citée en chat
  - [ ] 5.5 Section 4.4, un thème lecture seule refuse vraiment, sous-thème compris
  - [ ] 5.6 Section 4.5, une suppression ne laisse ni note ni citation en chat
  - [ ] 5.7 Section 4.6, la purge survit à un échec, mode avion puis relance
  - [ ] 5.8 Section 4.7, quitter en gardant ses notes, puis en les retirant
  - [ ] 5.9 Section 4.8, un membre révoqué cesse de recevoir
  - [ ] 5.10 Section 4.9, l'écho P2P ne laisse pas de vecteur orphelin
  - [ ] 5.11 Section 4.10, effacer mes données ne laisse aucun espace
  - [ ] 5.12 Noter le résultat de chaque section dans le bundle de vérification

- [ ] 6.0 Le gel en lecture seule _(brief: critère 7)_
  - [ ] 6.1 Retirer le premium au compte propriétaire avec les outils admin
  - [ ] 6.2 Vérifier que `pull` continue de servir et que toute écriture répond `space_read_only`
  - [ ] 6.3 Vérifier que l'app affiche ce gel au lieu de faire disparaître l'espace
  - [ ] 6.4 Rendre le premium et vérifier que l'écriture redevient possible

- [ ] 7.0 Clore la passe
  - [ ] 7.1 Corriger ce que le protocole a cassé, avant toute mise en ligne
  - [ ] 7.2 Pousser `feat/spaces-app` et ouvrir la PR vers `dev`
  - [ ] 7.3 Décider si un testeur tiers vaut le travail TestFlight, dans un brief séparé
