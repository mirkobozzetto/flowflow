---
type: brief
slug: spaces-test-setup
title: "Pouvoir tester les espaces partagés à deux comptes"
status: ready
created: 2026-08-24
---

# Pouvoir tester les espaces partagés à deux comptes

## En bref

Le code des espaces partagés est écrit des deux côtés, rien n'est validé sur
appareil. Le blocage n'est pas la distribution iOS : c'est qu'il n'existe qu'un
seul compte web, et que les deux appareils de Mirko sont dessus tous les deux.
Il faut un second compte, un appareil qui le porte, et le backend déployé.

## Problème

Un espace se joue entre deux identités web distinctes. Le serveur résout le
propriétaire et les membres par `web_user_id`, et le gate premium porte sur le
propriétaire, jamais sur l'appelant : c'est précisément ce qu'il faut prouver,
qu'un invité non payant participe.

L'état réel, lu dans le backend :

- Un seul compte web existe, de rôle admin. Il est premium, accordé en admin,
  sans expiration.
- Deux appareils sont enregistrés sur ce même compte, tous deux vus
  aujourd'hui : l'iPhone et l'app Mac.

Avec cette configuration, la moitié des scénarios est intestable. Un espace
créé par ce compte ne peut être rejoint par personne, puisqu'un membre est
défini par une identité web différente de celle du propriétaire.

Mirko pensait devoir publier l'app sur iOS pour tester à plusieurs. Ce n'est pas
nécessaire pour la première passe : deux appareils qu'il possède déjà suffisent,
à condition qu'ils portent deux comptes distincts. La distribution devient
nécessaire seulement pour faire tester une personne tierce.

## Objectif

Amener l'environnement à l'état où le protocole complet de
`verification-bundle-T07-T16.md` peut se dérouler du début à la fin, y compris
la révocation, la sortie et la propagation d'une suppression jusqu'au chat.

## Non-objectifs

- Publier sur TestFlight ou l'App Store. Utile pour un testeur tiers, pas pour
  cette passe.
- Créer une interface d'administration des comptes. Les outils admin existants
  suffisent.
- Automatiser le protocole. Il se déroule à la main, une fois.

## Ce qu'il faut mettre en place

### Le backend en production

La migration 18 et les onze routes `/v1/spaces/*` sont sur `main` depuis le
merge de la PR #90. Dokploy ne déploie pas tout seul : aucun webhook, aucun
workflow dans le dépôt. Tant que ce n'est pas fait, chaque appel d'espace
répond 404 et rien d'autre ne peut commencer.

### Un second compte web

Un second passkey sur `account.flowflow.be`, avec une adresse différente. Un
alias de la boîte principale suffit s'il arrive bien à destination, la
vérification passe par le lien de connexion.

### Un appareil qui porte ce second compte

L'app Mac joue le second appareil. Elle est aujourd'hui liée au compte
existant ; il faut lui rendre une identité neuve, puis lier cette identité au
second compte web depuis la page compte.

Effacer sa base locale lui donne un nouvel identifiant d'appareil au prochain
démarrage. C'est destructeur pour les notes locales du Mac : elles doivent être
sauvegardées avant, ou tenues pour jetables.

### La bonne répartition des rôles

L'iPhone garde le compte premium et joue le PROPRIÉTAIRE. Le Mac porte le
second compte et reste gratuit : c'est lui qui prouve qu'un invité non payant
participe pleinement.

### Le gel en lecture seule

Un scénario ne peut pas être joué sans toucher aux droits : quand le
propriétaire cesse de payer, l'espace gèle en lecture seule au lieu de
disparaître. Le retrait puis la remise du premium sur le compte propriétaire se
font avec les outils admin.

## Critères d'acceptation

- [ ] `GET /v1/spaces` répond autre chose qu'une erreur de route en production.
- [ ] Deux comptes web existent, avec deux adresses distinctes.
- [ ] Deux appareils sont enregistrés sur deux comptes DIFFÉRENTS, chacun avec
      son identité web liée.
- [ ] Le compte de l'iPhone est premium, celui du Mac ne l'est pas.
- [ ] Le Mac rejoint un espace créé par l'iPhone, sans jamais devenir premium.
- [ ] Les dix sections du protocole de vérification sont jouées, avec leur
      résultat noté.
- [ ] Le retrait du premium au propriétaire gèle l'espace en lecture seule ; le
      rendre le dégèle.

## Métrique de succès

Nombre de sections du protocole jouées jusqu'au bout, sur les dix.

Base actuelle : zéro, l'environnement ne le permet pas.
Cible : dix sur dix, dont obligatoirement la 4.5, zéro note fantôme après
suppression.
Fenêtre : la session de test qui suit le déploiement.

## Hors périmètre

- Android, Windows, Linux.
- Le test à plus de deux comptes.
- La montée en charge, les plafonds de vingt membres et cinq mille notes.
- L'invitation d'une personne réelle, qui suppose la distribution.

## Ce qui suit, si la passe est verte

Faire tester une personne tierce demande TestFlight, donc une build de
distribution et une soumission. C'est un travail distinct, à ouvrir seulement
une fois que le protocole passe à deux comptes.
