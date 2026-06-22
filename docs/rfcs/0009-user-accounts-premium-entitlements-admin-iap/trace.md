---
artifact: RFC 0009 (status Review, gate overridden by Mirko 2026-06-22)
scope: Q1.2c (join_token over Noise + /v1/account/join) + Q1.6 (account screen)
engine: solo
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped (code complete + all checks green; device install + backend deploy pending)
---

# Ship trace - RFC 0009 Q1.2c + Q1.6

## Definition of done

### Q1.2c - account join over the RFC 0004 Noise channel
- The joiner advertises its backend Ed25519 pubkey inside the existing
  `PairRequest`; the inviter (pairing host) mints a `join_token` via
  `POST /v1/account/invite` and returns it inside the existing `PairOk`.
- The joiner redeems it with `POST /v1/account/join` on its own session.
- Best-effort: pairing (RFC 0004 sync) still succeeds when no backend is
  configured, the runtime is unavailable, or the join call fails. No raw
  `account_id` is ever carried on the wire (server-bound token only).
- Rule: the device that SHOWS the code is the inviter (keeps its account);
  the device that SCANS folds into it (backend `join` handles the merge).

### Q1.6 - account / premium screen
- New `GET /v1/account` backend route -> `{ account_id, premium, device_cap,
  devices[] }`, gated by the device session, premium resolved by the shared
  `gate::is_premium` helper (env allowlist OR legacy flag OR active entitlement).
- New Settings > Account screen: account id, premium badge, members / cap,
  member device list, "leave account" (backend `leave`), "delete my data"
  (leave + local content wipe behind confirmation).

## Backend gap closed
- `GET /v1/account` did not exist (lib.rs had only invite/join/leave). Added.

## Files
- backend: `src/gate.rs`, `src/account.rs`, `src/lib.rs`
- app: `src/services/backend/mod.rs`, `src/services/sync/peers.rs`,
  `src/db/mod.rs`, `src/ui/state.rs`, `src/ui/settings/mod.rs`,
  `src/ui/settings/account.rs`, `src/services/i18n/locales/{fr,en}.ftl`
