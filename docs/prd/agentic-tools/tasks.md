---
feature: Outils agentiques sortants (webhook, MCP distant, Google Calendar)
slug: agentic-tools
type: tasks
source_prd: docs/prd/agentic-tools/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `apex` (or the implementer) to execute.

## Relevant Files
- `src/services/tools.rs` - rig Tool impls (TriggerWebhook, outils MCP distants, CreateCalendarEvent)
- `src/services/llm.rs` - câblage agent loop (prompt_with_agent / prompt_agent_with_tools)
- `src/services/constants.rs` - ajouts system prompt, endpoints MCP connus
- `src/services/error.rs` - variantes d'erreur outils (non bloquantes)
- `src/db/settings_repo.rs` - persistance URL+secret webhook, toggles/clés MCP, tokens Google
- `src/ui/settings.rs` - UI config (webhook, toggles MCP, bouton Connecter Google)
- `src/platform/ios/mod.rs` - FFI ASWebAuthenticationSession (OAuth), ouverture Réglages
- `Cargo.toml` - feature rig-core `rmcp` + crate `rmcp` à la version exacte pinnée

## Tasks

- [ ] 1.0 Webhook sortant  _(PRD: stories 1, 4, 5)_
  - [ ] 1.1 Settings: champ URL webhook + secret, persistés en SQLite (exclus de l'archive backup.)
  - [ ] 1.2 `TriggerWebhook` (rig Tool trait): POST JSON, en-tête `x-webhook-secret`, corps rempli par le modèle.
  - [ ] 1.3 Câbler le tool dans l'agent loop (disponible en chat.)
  - [ ] 1.4 Déclenchement: commande chat (manuel) en v1; documenter l'option d'événements auto pour plus tard.
  - [ ] 1.5 Échec: timeout + erreur non bloquante, message clair, le chat continue.

- [ ] 2.0 Outils MCP distants (Exa / Linear)  _(PRD: stories 2, 4, 5)_
  - [ ] 2.1 Valider la cross-compilation `rmcp` + transport Streamable HTTP sur aarch64-apple-ios + simulateur AVANT câblage.
  - [ ] 2.2 Cargo: `rig-core -F rmcp` + `rmcp` à la version exacte pinnée par rig 0.36 (`cargo tree -p rmcp`, éviter E0308.)
  - [ ] 2.3 Settings: liste de serveurs MCP avec toggle ON/OFF + clé Bearer par serveur (Exa, Linear.)
  - [ ] 2.4 Connexion `StreamableHttpClientTransport`, `list_tools`, injection `.rmcp_tools` dans l'agent.
  - [ ] 2.5 Réponse de test citant un résultat web (Exa) dans le chat.
  - [ ] 2.6 Échec/timeout d'un MCP: dégradation gracieuse, l'agent reste sur les outils locaux.

- [ ] 3.0 Google Calendar (note vers événement)  _(PRD: stories 3, 4, 5)_
  - [ ] 3.1 FFI `ASWebAuthenticationSession` (platform/ios), URL scheme `com.mirkobozzetto.flowflow://oauth` dans Info.plist.
  - [ ] 3.2 Flux auth-code + PKCE en Rust, échange du token via reqwest, `refresh_token` en Keychain/SQLite.
  - [ ] 3.3 `CreateCalendarEvent` (rig Tool): appelle l'API Calendar REST avec le bearer, depuis le contenu d'une note.
  - [ ] 3.4 Settings: bouton "Connecter Google" + état connecté/déconnecté + déconnexion.
  - [ ] 3.5 Échec auth/API: message clair, pas de crash, l'agent reste utilisable.

- [ ] 4.0 Sécurité des secrets  _(PRD: story 4)_
  - [ ] 4.1 Toutes les valeurs sensibles (secret webhook, clés MCP, tokens Google) en SQLite, jamais en clair dans le code.
  - [ ] 4.2 Étendre `SENSITIVE_KEYS` pour exclure ces clés de l'archive backup.
  - [ ] 4.3 Vérifier 0 secret loggé et 0 secret envoyé vers un host non prévu.

- [ ] 5.0 Tests & validation  _(PRD: acceptance criteria + success metrics)_
  - [ ] 5.1 Webhook: déclenchement reçu < 5 s, payload correct, secret présent en en-tête.
  - [ ] 5.2 MCP Exa: toggle ON, au moins 1 résultat web cité; OFF, outils locaux seulement.
  - [ ] 5.3 Calendar: note vers événement visible dans Google < 10 s; le refresh token survit.
  - [ ] 5.4 Dégradation: chaque outil externe down laisse le chat répondre, 0 crash.
  - [ ] 5.5 Cross-compile: build mobile vert sur les 3 cibles iOS avec `rmcp`.
