# Passe 2 - Spikes de faisabilité (T06 Noise, T08 appairage)

> But d'un spike: lever une inconnue AVANT d'investir dans l'implémentation complète. Ici, deux
> inconnues du RFC: (T06) `snow` cross-compile-t-il iOS et le handshake marche-t-il, et (T08)
> l'appairage QR/IP est-il viable. Le transport sur socket réel (T14) et l'UI d'appairage (T15)
> restent des tâches futures: ces spikes en posent les briques et prouvent qu'elles tiennent.

## Ce qui a été fait

### Nouveau module `src/services/sync/`
- `mod.rs` - `SyncError` (Handshake / Pairing / Transport), `Display` + `From<SyncError> for String`.
- `transport.rs` (T06) - Noise/PSK via `snow` 0.10.
- `peers.rs` (T08) - payload d'appairage + QR + repli IP.
- Déclaré par `pub mod sync;` dans `src/services/mod.rs` (seule modification d'un fichier existant).

### T06 - Noise `snow` XXpsk3
- Params: `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s`. PSK 32 octets mixée en position 3.
- API: `generate_static_keypair()`, `build_initiator(priv, psk)`, `build_responder(priv, psk)` -
  briques réutilisées telles quelles par T14.
- `inmemory_handshake_with_psks(...)` / `inmemory_handshake_roundtrip(...)` - exécutent le handshake
  3 messages (e / e,ee,s,es / s,se,psk), passent en mode transport AEAD, chiffrent puis déchiffrent
  un message. C'est la preuve de bout en bout du protocole, sans réseau.
- **Cross-compile iOS = la vraie inconnue, levée.** `snow` en `default-resolver` est 100% Rust
  (chacha20poly1305 + curve25519-dalek + blake2 + getrandom), **aucun cmake/C, aucun ring/aws-lc**.
  `make all` a compilé `snow`, `curve25519_dalek`, `chacha20poly1305` pour `aarch64-apple-ios` et
  installé l'app sur l'iPhone.

### T08 - appairage QR / IP
- `PairingPayload { device_id, addr, port, psk: [u8;32], static_pubkey }`.
- `encode_pairing_uri` / `decode_pairing_uri` - URI `flowflow://pair#<base64url(json)>` (tout ce
  qu'il faut pour qu'un pair se connecte + authentifie tient dans un QR).
- `generate_psk()` - 32 octets CSPRNG via `getrandom`.
- `pairing_qr_svg(uri)` - QR rendu en SVG (`qrcode` 0.14, feature `svg` seule, pas de crate `image`)
  -> directement affichable par l'UI en T15.
- `parse_manual_addr("ip:port")` - repli quand la caméra ne sert pas (saisie manuelle).
- **mDNS: décision déjà actée dans le RFC §6** - QR/IP en primaire (connexion TCP unicast = zéro
  entitlement multicast), Bonjour-système en v2, `mdns-sd` pur Rust exclu sur device (EHOSTUNREACH
  iOS 16+). Le spike ne ré-ouvre pas ce point.

### Dépendances ajoutées (`Cargo.toml`)
`snow` 0.10, `qrcode` 0.14 (no-default + `svg`), `base64` 0.22, `getrandom` 0.2. Toutes pures Rust,
cross-compilent iOS.

## Ce qui N'est PAS dans cette passe (volontaire)
- Pas de socket TCP, pas de framing length-prefixed -> c'est T14.
- Pas d'écran d'appairage, pas de scan caméra (AVFoundation) -> c'est T15.
- Aucune modification de la base, aucun trigger, aucun symbole existant touché (hors `pub mod sync;`).

## Comment vérifier en local

### Tests host (logique protocole + payload)
```
cargo test --test sync_transport_test --test sync_pairing_test
```
Attendu: 8 tests verts.
- `xxpsk3_handshake_roundtrip_succeeds` - handshake + AEAD round-trip.
- `xxpsk3_handshake_rejects_mismatched_psk` - PSK différente -> handshake refusé (anti-MITM).
- `static_keypair_has_curve25519_length` - clés 32 octets.
- `pairing_payload_roundtrips_through_uri` - encode puis decode redonne le même payload.
- `pairing_uri_fits_in_a_qr_code` - le payload rentre dans un QR (SVG généré).
- `decode_rejects_foreign_scheme` - une URL étrangère est rejetée.
- `generated_psks_differ` - deux PSK générées diffèrent.
- `manual_addr_parses_host_and_port` - repli IP `ip:port` parsé, entrées invalides rejetées.

### Cross-compile + device
```
make check     # fmt + clippy: 0 warning
make all       # build iOS + sign + install device (compile snow/qrcode/curve25519 pour iOS)
```
Attendu: `Compiled ... snow / qrcode / curve25519_dalek` puis `App installed`.

### Ce qui reste à valider plus tard (pas testable maintenant)
- Handshake sur socket TCP réel iPhone<->Mac (C6) -> arrive avec T14.
- Scan QR caméra + connexion réelle (C7) -> arrive avec T15.
Ces deux points exigent le transport réseau et l'UI, hors périmètre d'un spike.

## Suite possible (passe 3)
Deux pistes parallèles, au choix de Mirko:
- **Chaîne vecteurs (T09 -> T13)**: id de chunk déterministe, BLOB f32 dans `chunks`, reconstruction
  LanceDB sans ré-embed. Indépendante du transport, fort impact RAG.
- **Transport réel (T14 -> T15)**: Noise sur TCP + framing, puis écran d'appairage. Construit
  directement sur ces deux spikes.
