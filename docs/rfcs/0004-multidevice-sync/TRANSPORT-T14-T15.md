# Passe 5 - Transport Noise TCP + Appairage (T14/T15)

RFC 0004, chaîne transport. Premier lien réseau réel entre deux appareils.

## T14 - Transport Noise sur TCP

`src/services/sync/transport.rs`

- Pattern `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s` (snow, pur Rust, déjà prouvé iOS en passe 2).
- Framing length-prefixed: `u16` big-endian + payload, par frame Noise (<= 65535 octets).
- `SecureChannel<S>`: après handshake, `send`/`recv` en mode transport AEAD.
  - Message logique = header chiffré `u32` (longueur) + N frames de payload chunké par `MAX_PLAINTEXT` (65519 = 65535 - tag 16).
  - `recv` borne la taille (`MAX_LOGICAL_MESSAGE` 64 Mio), rejette toute frame sans progrès (anti-boucle) ou qui dépasse la longueur déclarée.
- `connect_secure` (initiator) / `accept_secure` (responder) sur `TcpStream`: timeouts lecture/écriture 20s, `TCP_NODELAY`.
- Vérification d'empreinte du static distant via `expected_remote_static`:
  - Côté initiator: faite APRÈS msg2 et AVANT l'envoi de msg3 -> on n'expose jamais notre clé statique à un mauvais pair.
  - Côté responder: après msg3.

Tests (`tests/sync_transport_test.rs`, 7): handshake in-memory, longueurs de clé, roundtrip TCP localhost, gros message 200 ko (multi-frames), PSK invalide refusée, empreinte responder invalide refusée, empreinte initiator invalide refusée côté responder.

## T15 - Appairage

`src/db/peer_repo.rs` (nouveau), `src/services/sync/peers.rs`, `src/ui/sync_pairing.rs` (nouveau).

- Table `sync_peers` (déjà en V10): CRUD + `persist_pairing`/`delete_pairing` ATOMIQUES (peer row + PSK en une transaction) -> jamais de pair sans PSK, jamais de secret orphelin.
- Identité statique du device persistée en `settings` (`sync_static_privkey`/`sync_static_pubkey`), générée une fois, stable.
- PSK par-pair en `settings` (`sync_psk_{device_id}`).
- `start_pairing_host`: bind TCP, payload QR (`flowflow://pair#<b64url(json)>` avec addr/port/PSK/static_pubkey), thread responder, fenêtre 300s, `cancel`.
- `join_pairing`: decode URI -> `connect_secure` (vérif empreinte) -> échange `pair_request`/`pair_ok` -> persiste le binding.
- `bind_peer` (garde sécurité): refuse un `device_id` vide; refuse d'écraser un pair existant dont la clé statique diffère (anti-hijack); un vrai re-appairage (même clé) garde l'état d'ack.
- `unpair`: supprime row + PSK (DELETE réel) atomiquement.
- UI `View::SyncPairing`: QR SVG + URI copiable + collage + liste des pairs + dissociation. Entrée via Settings -> Synchronisation.
- `NSLocalNetworkUsageDescription` ajouté (`Dioxus.toml`).
- Seam `FLOWFLOW_SYNC_ADVERTISE_ADDR` (annonce loopback en test, hermétique hors-ligne).

Tests (`tests/sync_pairing_test.rs`, 12): URI roundtrip, QR, identité stable, E2E host+join sur 2 DB, PSK falsifiée refusée, empreinte falsifiée refusée, hijack (clé différente même id) refusé + binding préservé, unpair nettoie row+PSK.

## Revue adversariale (ultracode, 4 lentilles, 8 agents)

3 MAJOR confirmés + 5 robustesses corrigés AVANT commit. 1 MAJOR réfuté (TOCTOU identité = fenêtre de quelques ms, non atteignable par interaction humaine). Mineurs/NITs UX et durcissement reportés à T17+ (voir notes du trace).

## Ce que ça NE fait PAS encore

L'appairage écrit seulement l'annuaire des pairs + l'identité/PSK. AUCun échange de notes: c'est le protocole T17 (HELLO/PUSH/ACK). Les deux apps savent désormais se trouver et ouvrir un canal chiffré authentifié; elles ne se synchronisent pas encore.
