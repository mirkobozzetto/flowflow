# Verification bundle - RFC 0009 Q1.5 admin console

Status: BUILT + VALIDATED by Claude (Mirko said "lance tout toi-meme").
Install, shadcn components, typecheck, biome, build, Docker image, and the
proxy-to-prod path are all green. What remains for you: a click-test with
the real ADMIN_TOKEN, and the Dokploy deploy if wanted.

## 1. Install + shadcn components (DONE)

Already run; `bun.lock` + `src/components/ui/*` are committed.
```bash
cd marketplace-flowflow/admin
bun install                                  # 138 pkgs, pins resolved
# components were pulled once via:
# bunx shadcn@latest add button input label card table badge sonner
```

## 2. Static gates (DONE - all green)

```bash
bun run typecheck   # tsc strict      -> 0 errors
bun run check       # biome 80 cols   -> 0 errors
bun run build       # vite build      -> client + SSR OK
```

## 3. Run against a backend

Backend must be reachable with `ADMIN_TOKEN` set.

- Local: `marketplace-flowflow` running on :8080, `ADMIN_TOKEN` in its
  `.env`. Admin uses the default `BACKEND_URL=http://localhost:8080`.
- Production: `cp .env.example .env` then set
  `BACKEND_URL=https://api.flowflow.be`.

```bash
bun dev             # http://localhost:3000
```

## 4. Manual flow (the acceptance checks)

| # | Do | Expect |
|---|----|--------|
| C2 | Paste the `ADMIN_TOKEN`, Sign in | Lands on /dashboard |
| C2 | Wrong token | Toast "Invalid admin token", stays on login |
| C3 | DevTools > Application > Cookies on localhost:3000 | `admin_session` cookie present on :3000 (not :8080) |
| C4 | Devices card | Lists registered devices (id, account, last seen) |
| C5 | Entitlements card | Lists entitlements newest first |
| C6 | Target = Account id (or Device pubkey), Grant | Toast "Granted"; row shows active/admin |
| C7 | Same target, Revoke | Toast "Revoked"; row status revoked |
| C8 | Clear sessionStorage, reload /dashboard | Bounced to login |

## 5. Real use right now

To grant premium on prod without deploying the admin: set
`BACKEND_URL=https://api.flowflow.be`, `bun dev`, sign in with the prod
`ADMIN_TOKEN` (Dokploy > Environment), find the device in Devices, Grant.

## 6. Docker / Dokploy (VALIDATED end to end vs prod)

```bash
docker build -t flowflow-admin ./admin
docker run -p 3000:3000 -e BACKEND_URL=https://api.flowflow.be flowflow-admin
```
Verified against `https://api.flowflow.be` through the container's proxy:
GET / -> 200 (login SSR), GET /v1/admin/devices -> 401, POST /login wrong
token -> 403. Dokploy: "Compose" app from this folder, attach a domain to
the `admin` service (port 3000, HTTPS), set `BACKEND_URL`.

Note when scripting a readiness probe: the server boots in ~1-2s; use
`curl --retry-all-errors`, not `--retry-connrefused` (it won't retry the
startup reset).
