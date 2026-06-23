---
artifact: RFC 0009 (status Review, gate overridden by Mirko)
scope: Q1.5 - TanStack Start + shadcn admin shell
engine: solo
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: shipped - built, all gates green, Docker validated vs prod
---

# Ship trace - RFC 0009 Q1.5 admin console

## Definition of done
See contract.md. Thin React shell over the 7 live /v1/admin routes.
Zero backend change. New dir `marketplace-flowflow/admin/`.

## Backend API consumed (read-only, already deployed)
- POST /v1/admin/login {token} -> Set-Cookie admin_session + {csrf}
- POST /v1/admin/logout, POST /v1/admin/refresh
- GET  /v1/admin/devices
- GET  /v1/admin/entitlements[?account_id=]
- POST /v1/admin/entitlements/grant  (x-csrf-token)
- POST /v1/admin/entitlements/revoke (x-csrf-token)

## Files created (all under marketplace-flowflow/admin/)
- package.json - deps (TanStack Start 1.168, Router 1.170, React 19,
  Tailwind v4, Biome 2, cva/clsx/tailwind-merge). shadcn comps via CLI.
- tsconfig.json - strict + paths @/*.
- biome.json - lineWidth 80, single quotes, no semicolons.
- vite.config.ts - tanstackStart() + viteReact() + tailwindcss(), @ alias.
- components.json - shadcn config (new-york, neutral, css vars).
- src/styles.css - Tailwind v4 + shadcn tokens; primary = FlowFlow orange.
- src/router.tsx - getRouter().
- src/lib/utils.ts - cn().
- src/lib/api.ts - typed client; csrf in sessionStorage; AuthError on 401.
- src/routes/__root.tsx - document shell + Sonner Toaster.
- src/routes/v1/admin/$.ts - server route: same-origin proxy to backend.
- src/routes/index.tsx - login (ADMIN_TOKEN).
- src/routes/dashboard.tsx - grant/revoke form + entitlements + devices.
- .env.example, .gitignore, README.md.

## Auth model (no auth lib)
ADMIN_TOKEN -> POST /login -> httpOnly cookie (proxied to :3000) + csrf in
sessionStorage -> x-csrf-token on mutations. 401 -> clear + back to login.
On reload the csrf survives in sessionStorage; if the cookie expired the
first call 401s back to login.

## Decisions
- Location: subdir `admin/` in marketplace-flowflow (user-confirmed).
- Proxy over CORS: backend has no CORS layer; the TanStack server route
  forwards server-side so the cookie is first-party. No backend change.
- shadcn components pulled via `shadcn add` (official, version-correct)
  rather than hand-vendored, to dodge RC version drift.
- Catalog screen deferred: no backend endpoint exists.

## Build + validation actually run (Mirko said "lance tout toi-meme")
- `bun install`: 138 pkgs. Pins resolved: react-start 1.168.26, router
  1.170.16, react 19.2.7, vite 6.4.3, tailwind 4.3.1, biome 2.5.0.
- `shadcn add button input label card table badge sonner`: 7 components in
  src/components/ui (committed; not vendored by hand). Pulled lucide-react,
  radix-ui, sonner, next-themes.
- `bun run typecheck` (tsc strict): 0 errors.
- `bun run check` (biome, lineWidth 80): 0 errors. Added
  `css.parser.tailwindDirectives` so Biome accepts the Tailwind v4 @apply.
- `bun run build`: client + SSR bundles OK.

## Prod server entrypoint (the one snag, resolved)
`vite build` emits a passive WinterCG `export default { fetch }` at
dist/server/server.js - it does NOT listen. `serve.ts` hosts it via
`Bun.serve({ hostname: '0.0.0.0', port })`. The build has no listen/host
logic of its own (only TSS_SHELL/TSS_PRERENDERING). The repeated "it fails"
was a TEST-HARNESS race: the server boots in ~1-2s (SSR prints a "Log
Summary" banner) and curl's `--retry-connrefused` does not retry the reset
in that window; `--retry-all-errors` waits correctly.

## Docker / Dokploy (validated end to end vs PROD)
- `Dockerfile` (multi-stage oven/bun:1.3.14): build (full deps) -> runtime
  (prod deps + dist + serve.ts). Runtime needs node_modules (SSR bundle
  externalizes h3 etc.). `docker build` OK, image ~534MB.
- `compose.yml`: Dokploy "Compose" service `admin`, port 3000,
  dokploy-network, BACKEND_URL env.
- Container run with BACKEND_URL=https://api.flowflow.be:
  GET / -> 200 (login SSR); GET /v1/admin/devices -> 401 unauthorized (from
  the REAL Rust backend through the proxy); POST /v1/admin/login wrong token
  -> 403 forbidden. Proxy + cookie path proven against production.

## Remaining for Mirko
- `bun dev` and click-test grant/revoke with the REAL ADMIN_TOKEN against a
  reachable backend (prod or a local marketplace-flowflow on a free port;
  :8080 is currently taken by another local app "Murmur").
- Deploy the `admin` Dokploy service + attach admin.flowflow.be if wanted.
- Uncommitted; no git action taken (no approval).
