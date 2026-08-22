# FlowFlow Account

Customer account site for `account.flowflow.be` (RFC 0025). Astro SSR
(`output: "server"`, `@astrojs/node`), a one-way fork of the landing's
`/beta` design system (`tokens.css`, `beta.css`, the dashboard panes).
The landing stays the source of truth for the visual language.

## Run

From the repo root:

```bash
make account
```

Or here:

```bash
bun install
ACCOUNT_PREVIEW=1 bun run dev   # fixture data, no backend needed
bun run dev                     # real mode, needs BACKEND_URL
bun run build                   # production build (dist/)
```

Open <http://localhost:4321/> (EN) or <http://localhost:4321/fr/>.

`ACCOUNT_PREVIEW=1` swaps in fixture data so the design can be reviewed
with nothing else running. Without it, pages authenticate against the
backend and redirect to `/login` when no session cookie is present.

## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `BACKEND_URL` | `http://localhost:8080` | marketplace backend origin |
| `ACCOUNT_PREVIEW` | unset | `1` = fixture data, skip auth |

## Structure

- `src/pages/` - dashboard (`index`), `login`, `register`, `link`,
  plus `fr/` variants
- `src/pages/v1/` - four narrow same-origin proxies to the backend:
  `/v1/auth`, `/v1/me`, `/v1/requests`, `/v1/account/link`.
  Never add a `/v1` catch-all: it would expose `/v1/admin/*`.
- `src/lib/proxy.ts` - the forward (multiple `Set-Cookie` via
  `getSetCookie()`, `cache-control: no-store` on `/v1/me/*`,
  `x-forwarded-for` appended)
- `src/lib/api.ts` - server-side reads + preview fixtures
- `src/components/Dashboard.astro` - the five panes: overview,
  devices, services, billing, security
- `src/scripts/auth.ts` - WebAuthn ceremonies
  (`@simplewebauthn/browser`) for login, register, device link
- `src/styles/` - copied from the landing (one-way fork)

## Auth model

One passkey per user, valid on every `flowflow.be` site
(`RP_ID=flowflow.be`, RFC 0025 6.1). The session cookie is host-only,
`SameSite=Strict`, kept first-party by the same-origin proxy. Login
translates the client-side `NotAllowedError` into "no passkey on this
device"; the server never reveals whether an email exists.

## Deploy

`Dockerfile` builds with bun and runs `dist/server/entry.mjs` on
Node 22 (port 4321). Deployed as its own Dokploy service on
`account.flowflow.be` with `BACKEND_URL` pointing at the backend.
