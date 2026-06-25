# M1.17 activation - prod ops (run by Mirko)

Device code is shipped: `flowflow src/domain/agent_manifest.rs` pins the production
`ADMIN_PUBKEY = ed25519:7xalCAYJE6u/ydk7ruheDUnO+6YAiK8X2Y6If2CCXoE=`. Compiles clean.

The matching private seed (32-byte hex) was generated and written OUT of the repo:
`<scratchpad>/AGENT_SIGNING_KEY.prod.secret`. It was never printed to chat.
**Dokploy is its permanent home - paste it there before anything, then the file can go.**
If this seed is lost and only the pubkey stays pinned, the device trusts a signer that
no longer exists -> no agent ever installs.

## 1. Deploy the signing key (Dokploy env, then restart)

```
AGENT_SIGNING_KEY=<paste the hex from AGENT_SIGNING_KEY.prod.secret>
AGENT_SIGNER_KEY_ID=prod-admin
```

Restart the backend. At boot it logs `agent signing enabled: ... public_key=ed25519:...`.
That logged pubkey MUST equal `7xalCAYJE6u/ydk7ruheDUnO+6YAiK8X2Y6If2CCXoE=`. If not, stop.

## 2. Activate the agent (hidden -> active)

Prod DB:

```sql
UPDATE catalog_items SET status = 'active' WHERE id = 'agent-crm-sync';
```

## 3. Grant your device (premium)

```
curl -X POST https://api.flowflow.be/v1/admin/entitlements/grant \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"device_pubkey":"<your device pubkey>"}'
```

Double-submit just re-activates; harmless if already granted.

## 4. Force a real fetch (the actual test)

Reinstalling over the same bundle id keeps SQLite, so the OLD pin is reused and the device
never fetches. To force the fetch:

1. Delete FlowFlow from the iPhone (wipes its SQLite -> no pin).
2. Claude runs `make all` (fresh install).
3. Re-login, arm the agent on device.

Expected: device hits `GET /v1/agents/agent-crm-sync/package`, verifies the signature
against the pinned prod pubkey, pins it, then talks to your Excel.
Success = Excel works AND it came from the backend, not the hardcoded fixture.
