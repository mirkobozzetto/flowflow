---
feature: Serveur LAN, exposer notes et backup sur le réseau local
slug: lan-serve
type: tasks
source_prd: docs/prd/lan-serve/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `apex` (or the implementer) to execute.

## Relevant Files
- `src/services/serve.rs` - nouveau module serveur HTTP local (routes, auth token, cycle de vie)
- `src/services/backup.rs` - réutilise l'archive export pour la route de téléchargement
- `src/db/note_repo.rs` - lecture et recherche des notes pour les routes
- `src/platform/ios/mod.rs` - FFI isIdleTimerDisabled, annonce mDNS/Bonjour, ouverture Réglages
- `src/ui/settings.rs` - toggle Serve mode, affichage URL + token + état
- `Cargo.toml` - crate serveur HTTP (tiny_http ou axum) + éventuel mDNS

## Tasks

- [ ] 1.0 Serveur HTTP local + lecture des notes  _(PRD: stories 1, 2)_
  - [ ] 1.1 Module `serve.rs`: bind sur un port, démarrage/arrêt pilotés par un toggle.
  - [ ] 1.2 Route `GET /notes` (liste JSON) + `GET /note/{id}`.
  - [ ] 1.3 Route `GET /search?q=` renvoyant les mêmes résultats que l'app.
  - [ ] 1.4 Page d'index HTML minimale lisible au navigateur (liste + recherche.)

- [ ] 2.0 Session stable (no-veille + cycle de vie)  _(PRD: story 4 + transversal)_
  - [ ] 2.1 `isIdleTimerDisabled = true` quand Serve ON; reset à OFF.
  - [ ] 2.2 Couper le serveur quand l'app passe en arrière-plan; rouvrir au retour au premier plan.
  - [ ] 2.3 Auto-coupure après N minutes sans requête: serveur OFF + écran peut se rendormir.
  - [ ] 2.4 Bandeau de guidage Concentration/Mode Avion + bouton ouvrir Réglages (l'app ne bloque pas les appels.)

- [ ] 3.0 Sécurité d'accès  _(PRD: story 6)_
  - [ ] 3.1 Token requis sur chaque route (en-tête ou query); sans token valide, 401.
  - [ ] 3.2 Token généré par session, affiché dans Settings; option régénérer.
  - [ ] 3.3 LAN uniquement par défaut; avertissement pour wifi public (pas de TLS en v1.)

- [ ] 4.0 Découverte réseau  _(PRD: story 5)_
  - [ ] 4.1 Valider mDNS/Bonjour en pur Rust sur iOS; sinon repli sur l'IP affichée.
  - [ ] 4.2 Annoncer `flowflow.local` + le port; afficher l'URL et le token dans Settings.

- [ ] 5.0 Téléchargement du backup par le réseau  _(PRD: story 3)_
  - [ ] 5.1 Route `GET /backup.zip` servant l'archive du PRD data-backup-export.
  - [ ] 5.2 Générer l'archive à la demande (staging) puis la streamer (io::copy, pas de lecture complète en mémoire.)
  - [ ] 5.3 Intégrité: aller-retour identique à l'export local.

- [ ] 6.0 (Stretch) Relais 5G hors LAN  _(PRD: story 7)_
  - [ ] 6.1 Évaluer un relais sortant (tel vers VPS vers client) en option séparée; chiffrer coût et scope.
  - [ ] 6.2 Décision: inclure ici ou créer un PRD relais dédié.

- [ ] 7.0 Tests & validation  _(PRD: acceptance criteria + success metrics)_
  - [ ] 7.1 Mac même wifi: `/notes` < 2 s; recherche en parité avec l'app.
  - [ ] 7.2 Sans token: 401, 0 note exposée.
  - [ ] 7.3 Écran reste allumé Serve ON; auto-coupure puis l'écran se rendort.
  - [ ] 7.4 App en arrière-plan: serveur coupé, 0 socket fantôme.
  - [ ] 7.5 `/backup.zip` intègre, < 30 s pour le volume cible.
