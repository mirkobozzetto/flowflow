---
feature: Serveur LAN, exposer notes et backup sur le réseau local
slug: lan-serve
type: prd
status: ready
built: no
stepsCompleted: [0, 1, 2, 3, 4]
---

> NOTE (2026-06-11): spec finalisee (status: ready) mais NON implementee - zero
> code dans `src/`. "ready" = pret a construire, pas construit. Lancer
> `/ship docs/prd/lan-serve/` pour implementer (ou `/rfc` d'abord si le design technique doit etre tranche).

# PRD: Serveur LAN, exposer notes et backup sur le réseau local

## Problem statement

Les notes vivent dans l'app, consultables seulement sur le petit écran du téléphone.
Aucun moyen simple de lire ou chercher ses notes depuis le Mac, ni de récupérer
l'archive de backup sans passer par le share sheet ou un câble.

iOS interdit un serveur entrant persistant, mais autorise un serveur HTTP local tant
que l'app est au premier plan, joignable par les appareils du même wifi (démontré par
la recherche de faisabilité: WorldWideWeb, Working Copy, iSH le font.) C'est exactement
le créneau exploitable: app ouverte plus réseau local égale serveur joignable.

Pourquoi maintenant: la fonctionnalité de backup (PRD data-backup-export) produit déjà
une archive; l'exposer en téléchargement réseau est un complément naturel, et lire ses
notes sur grand écran est une demande récurrente de Mirko.

## Goals

- Exposer les notes en lecture et recherche sur le réseau local (Mac, navigateur),
  app au premier plan.
- Télécharger l'archive de backup directement par le réseau, sans share sheet ni câble.
- Garder la session stable: écran non mis en veille pendant le service, découverte
  simple par nom, coupure propre quand l'app quitte le premier plan.
- Protéger l'accès par un token: aucune note exposée sans le consentement explicite.
- Optionnel: joindre le téléphone hors-LAN (5G) via un relais sortant.

## Non-goals / Out-of-scope

- Pas de serveur persistant ni d'arrière-plan: le serveur vit seulement quand l'app est
  au premier plan (suspension iOS = serveur coupé, documenté.)
- Pas de synchronisation multi-appareil ni d'écriture massive: lecture et téléchargement
  d'abord; l'écriture depuis le Mac (POST note) est un stretch, pas le coeur.
- Pas de reachability internet par défaut: LAN uniquement. Le relais 5G est une option
  séparée qui sort du 100% offline.
- Pas de blocage des appels entrants par l'app (impossible iOS): guidage manuel seulement
  (Concentration, Mode Avion.)
- Pas de TLS dans la v1 LAN (localhost/LAN); à noter pour un usage sur wifi public.

## User stories

1. **Lire les notes via wifi.** En tant qu'utilisateur, je veux ouvrir mes notes dans le
   navigateur du Mac sur le même wifi, afin de lire et copier sur grand écran.
2. **Recherche.** En tant qu'utilisateur, je veux chercher mes notes via une URL, afin de
   retrouver vite une note depuis le Mac.
3. **Télécharger le backup.** En tant qu'utilisateur, je veux récupérer l'archive de
   backup par le réseau, afin d'éviter le share sheet ou le câble.
4. **Session stable.** En tant qu'utilisateur, je veux que l'écran ne s'éteigne pas
   pendant que je sers, afin que la connexion ne tombe pas.
5. **Découverte.** En tant qu'utilisateur, je veux joindre le téléphone par un nom simple
   (flowflow.local), afin de ne pas taper l'adresse IP.
6. **Sécurité d'accès.** En tant qu'utilisateur, je veux un accès protégé par token, afin
   que personne d'autre sur le wifi ne lise mes notes.
7. **(Stretch) Relais 5G.** En tant qu'utilisateur, je veux joindre mon téléphone hors
   wifi, afin que ça marche aussi en mobilité.

## Acceptance criteria

**Story 1 (lire)**
- Given l'app au premier plan et le Mac sur le même wifi, When j'ouvre l'URL du serveur,
  Then la liste de mes notes s'affiche dans le navigateur.

**Story 2 (recherche)**
- Given des notes présentes, When je recherche via l'URL, Then les résultats sont
  identiques à ceux de l'app (aucun écart.)

**Story 3 (backup)**
- Given une archive disponible, When je la télécharge par le réseau, Then le fichier est
  intègre et identique à un export local (aller-retour fidèle.)

**Story 4 (session)**
- Given le serve mode activé, When le temps passe, Then l'écran reste allumé.
- Given une inactivité prolongée, When le délai est atteint, Then le serveur se coupe et
  l'écran peut se rendormir.

**Story 5 (découverte)**
- Given le serve mode actif, When je cherche le téléphone, Then il est joignable par un
  nom (flowflow.local) ou, à défaut, par l'IP affichée dans l'app.

**Story 6 (sécurité)**
- Given un token requis, When une requête arrive sans token valide, Then elle est refusée
  (401) et aucune note n'est exposée.

**Story 7 (stretch, relais 5G)**
- Given le relais activé et le téléphone sur 5G, When je joins le téléphone hors wifi,
  Then la connexion passe par un relais sortant (aucun port entrant ouvert sur le tel.)

**Cycle de vie (transversal)**
- Given le serve mode actif, When l'app passe en arrière-plan, Then le serveur se coupe
  proprement (aucun socket fantôme), And reprend au retour au premier plan.

## Success metrics

- Mac sur le même wifi: la liste des notes s'affiche en moins de 2 s après ouverture
  de l'URL.
- Recherche réseau: 0 écart de résultats par rapport à l'app.
- Archive téléchargée intègre, aller-retour identique à l'export local, en moins de 30 s
  pour le volume cible.
- Écran reste allumé tant que serve mode ON; auto-coupure après le délai d'inactivité.
- Accès sans token: 100% refusés (401), 0 note exposée.
- App passée en arrière-plan: serveur coupé en moins du délai de grâce iOS, 0 socket
  fantôme.

## Constraints & assumptions

- iOS uniquement, 100% Rust/Dioxus, tokio déjà présent.
- Serveur HTTP local, joignable seulement app au premier plan et sur le même wifi (LAN);
  la suspension iOS coupe le serveur (documenté.)
- L'app ne peut pas bloquer les appels: guidage manuel Concentration/Mode Avion, plus un
  rappel que sur 5G un appel ne coupe pas la data (VoLTE) tant qu'on ne décroche pas.
- Sur 5G il n'y a pas de LAN avec le Mac: un relais sortant (VPS) est requis, ce qui en
  fait une option séparée.
- Réutilise l'archive du PRD data-backup-export pour le téléchargement.

## Open questions

- Port fixe (8080) ou port aléatoire affiché dans l'app ?
- Token: code court affiché vs lien complet contenant le token ?
- mDNS/Bonjour disponible en pur Rust sur iOS à valider, sinon repli sur l'IP brute ?
- Relais 5G: que faut-il héberger côté VPS, quel coût, et est-ce dans ce PRD ou un PRD
  relais dédié ?
- Écriture depuis le Mac (POST note) dans cette version ou plus tard ?
