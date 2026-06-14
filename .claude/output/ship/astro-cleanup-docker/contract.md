---
type: contract
kind: inline
slug: astro-cleanup-docker
created: 2026-06-13
status: locked
---

# Ship Contract: Astro landing cleanup + Docker for Dokploy

## CONTEXT

- Stack: Astro 6 (static output, no SSR adapter), Bun (bun.lock), Tailwind v4, Biome, TypeScript strict.
- Landing lives in `landing-page/` inside the flowflow monorepo.
- Biome linter is DISABLED for `.astro` files (override in biome.json); active on `.ts`.
- Branch: main.

## TASK

1. Remove unused variables and unused imports in the Astro landing.
2. Add Docker support so the landing can be deployed on a VPS via Dokploy.

## FINDINGS (evidence-based scan)

- `astro check` (clean), Biome on `.ts` (clean), and `astro check` WITH `noUnusedLocals`+`noUnusedParameters`
  injected (clean, 0 hints over 32 files): the Astro files contain NO unused locals, params, or imports.
- The one genuine dead symbol: `getLangFromUrl` in `src/lib/i18n.ts` (exported, never imported in code;
  referenced only in `src/CONTRACT.md` prose). `noUnusedLocals` cannot catch it because it is exported.
- `isLang` is exported and used internally by `resolveLang`; documented as public helper in CONTRACT.md -> keep.

## SCOPE OF EDITS

- `landing-page/src/lib/i18n.ts` (remove dead export)
- `landing-page/src/CONTRACT.md` (sync doc wording for removed symbol)
- NEW: `landing-page/Dockerfile`
- NEW: `landing-page/.dockerignore`
- NEW: `landing-page/nginx.conf`
- Optional NEW: `landing-page/docker-compose.yml` (local parity / Dokploy compose mode)

## DEFINITION OF DONE (acceptance)

- [ ] A1: No genuinely unused export/import/var remains in the Astro landing; `getLangFromUrl` removed and
      CONTRACT.md updated to match. `bun run check` still reports 0 errors.
- [ ] A2: `landing-page/Dockerfile` does a multi-stage Bun build -> nginx:alpine, serves static `dist/` on
      port 80 (the port Dokploy expects).
- [ ] A3: nginx config handles SPA-ish static routing, the `/fr/` locale path, and sane caching of hashed assets.
- [ ] A4: `.dockerignore` excludes node_modules/dist/.astro so the build context is lean.
- [ ] A5: A short deploy note documents the Dokploy settings (Dockerfile build, port 80).

## OUT OF SCOPE

- Switching Astro to SSR or adding a Node adapter.
- Touching the Rust app, Makefile, or anything outside `landing-page/`.
- Running the actual deploy on the VPS (user runs it).
