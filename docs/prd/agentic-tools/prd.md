---
feature: Outils agentiques sortants (webhook, MCP distant, Google Calendar)
slug: agentic-tools
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
---

# PRD: Outils agentiques sortants (webhook, MCP distant, Google Calendar)

## Problem statement

L'agent de FlowFlow (chat RAG) n'agit que sur les notes locales. Il ne peut ni
agir sur le monde extérieur, ni tirer de connaissance externe en direct. Toute la
valeur reste enfermée dans l'appareil.

Mirko veut que FlowFlow devienne agentique au sens fort: déclencher ses automations
(n8n, Make, Zapier), chercher le web en parallèle de ses notes, et transformer une
note en action datée (événement Google Calendar.) Ces trois besoins partagent une
même direction technique, la seule faisable sur iOS: le sortant. L'app appelle vers
l'extérieur pendant qu'elle est au premier plan, jamais l'inverse (cf. la recherche
de faisabilité iOS: aucun serveur entrant persistant possible.)

Pourquoi maintenant: l'agent multi-provider (rig-core) et le client HTTP (reqwest)
sont déjà en place. Le coût marginal pour brancher des outils sortants est faible,
et c'est le plus gros levier produit pour un effort réduit.

## Goals

- Permettre à l'agent de declencher une automation externe (webhook) depuis le chat.
- Permettre à l'agent d'utiliser des outils distants (recherche web, etc.) via des
  serveurs MCP activables en toggle.
- Permettre de transformer une note en événement Google Calendar.
- Rester dans le modèle iOS faisable: 100% sortant, app au premier plan, zéro
  dépendance à un serveur entrant ou à de l'arrière-plan.
- Ne jamais exposer de secret: URLs, tokens et clés restent locaux sur l'appareil.
- Dégradation gracieuse: un outil externe indisponible ne casse jamais le chat.

## Non-goals / Out-of-scope

- Pas de serveur entrant ni de reachability du téléphone depuis l'extérieur
  (impossible iOS, démontré par la recherche de faisabilité.)
- Pas d'App Intents / Siri (Swift uniquement, casse la contrainte 100% Rust.)
- Pas d'exécution en arrière-plan: l'agent agit quand l'app est ouverte.
- Pas d'intégration Apple Notes (aucune API publique iOS.)
- Pas de serveurs MCP en stdio / process local (impossible iOS): uniquement MCP
  distant via HTTP (Streamable HTTP.)
- Pas d'OAuth 2.1 pour les MCP qui l'exigent (GitHub, Notion) dans cette version:
  seulement les serveurs à clé statique (Exa, Linear.)
- Pas de marketplace de serveurs MCP ni de configuration avancée multi-tenant.

## User stories

1. **Webhook sortant.** En tant qu'utilisateur, je veux qu'une commande du chat
   déclenche un webhook n8n/Make, afin de brancher FlowFlow sur mes automations.
2. **Outils MCP distants.** En tant qu'utilisateur, je veux activer des serveurs MCP
   (Exa) en toggle, afin que l'agent cherche le web en parallèle de mes notes.
3. **Note vers Calendar.** En tant qu'utilisateur, je veux transformer une note en
   événement Google Calendar, afin que mes idées deviennent des actions datées.
4. **Secrets locaux.** En tant qu'utilisateur, je veux que mes URLs, tokens et clés
   restent sur l'appareil, afin que rien ne fuite et que mes archives restent saines.
5. **Robustesse offline.** En tant qu'utilisateur, je veux qu'un outil externe en
   échec ne casse pas le chat, afin de garder l'agent utilisable même réseau coupé.

## Acceptance criteria

**Story 1 (webhook)**
- Given une URL de webhook et un secret configurés, When je demande à l'agent de
  déclencher le webhook, Then une requête POST part avec le secret en en-tête et un
  corps JSON, And la cible (n8n/Make) reçoit l'appel.
- Given aucun webhook configuré, When je tente le déclenchement, Then l'agent répond
  clairement qu'il faut le configurer, sans erreur bloquante.

**Story 2 (MCP distant)**
- Given le serveur Exa MCP activé avec sa clé, When je pose une question nécessitant
  le web, Then l'agent utilise l'outil distant et cite au moins un résultat web.
- Given le serveur MCP désactivé, When je pose la même question, Then l'agent reste
  sur ses outils locaux sans erreur.

**Story 3 (Calendar)**
- Given un compte Google connecté, When je demande de transformer une note en
  événement, Then l'événement apparaît dans Google Calendar.
- Given un compte non connecté, When je tente la création, Then l'agent invite à
  connecter Google, sans crash.

**Story 4 (secrets)**
- Given des secrets enregistrés, When j'inspecte le code et une archive de backup,
  Then aucun secret n'y figure (tous en stockage local exclu de l'archive.)

**Story 5 (robustesse)**
- Given un outil externe injoignable (timeout/erreur), When l'agent l'appelle, Then
  le chat répond quand même avec un message clair, And l'app ne crashe pas.

## Success metrics

- Un webhook configuré se déclenche et la cible reçoit le payload en moins de 5 s.
- 0 secret présent dans le code source ou dans une archive exportée (vérifiable.)
- Toggle Exa MCP activé: une réponse de test cite au moins 1 résultat web.
- Note transformée en événement Calendar visible dans Google en moins de 10 s.
- 100% des outils externes en échec laissent le chat répondre, 0 crash observé.
- Build mobile vert sur les 3 cibles iOS avec la dépendance MCP ajoutée.

## Constraints & assumptions

- iOS uniquement, 100% Rust/Dioxus, agent actif seulement app au premier plan.
- rig-core 0.36 + reqwest déjà présents; la cross-compilation de la couche MCP doit
  être validée sur aarch64-apple-ios + simulateur AVANT tout câblage UI.
- MCP distant via HTTP (Streamable HTTP) uniquement; pas de stdio/process local.
- MCP à clé statique d'abord (Exa, Linear); OAuth 2.1 hors scope.
- Google: flux auth-code + PKCE (le flux loopback est déprécié sur iOS); pas de SDK
  Google Sign-In (casserait le 100% Rust.)
- Ordre de livraison imposé: webhook, puis MCP distant, puis Google Calendar.

## Open questions

- Le webhook se déclenche-t-il uniquement à la demande (commande chat) en v1, ou
  aussi sur des événements auto (création de note, génération de tags) plus tard ?
- Un seul webhook nommé ou plusieurs cibles configurables ?
- Calendar: prévoir un repli EventKit local (write-only, sans OAuth) si le flux PKCE
  s'avère trop lourd ?
- Serveurs MCP: liste blanche curée dans l'app vs URL libre saisie par l'utilisateur
  (risque de pointer vers un serveur non fiable) ?
