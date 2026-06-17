---
type: trace
kind: inline
slug: astro-cleanup-docker
engine_tier: solo
created: 2026-06-13
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped
---

# Trace Ledger: astro-cleanup-docker

| Task | Description | Status | Files | Notes |
|------|-------------|--------|-------|-------|
| T1 | Remove dead export `getLangFromUrl` + sync CONTRACT.md | done | src/lib/i18n.ts, src/CONTRACT.md | only genuinely dead symbol; astro check 0 errors after |
| T2 | Add Dockerfile (Bun build -> nginx serve dist on :80) | done | Dockerfile | oven/bun:1-alpine + nginx:1.27-alpine, HEALTHCHECK |
| T3 | Add nginx.conf (static routing, /fr/, asset caching) | done | nginx.conf | try_files dir-format; immutable _astro+fonts; no-cache HTML |
| T4 | Add .dockerignore (lean build context) | done | .dockerignore | excludes node_modules/dist/.astro/git |
| T5 | Add docker-compose.yml + deploy note | done | docker-compose.yml, README.md | compose maps 8080:80; README Dokploy section |

## Verification (run by lead, all SAFE)

- `bun run check` -> 0 errors, 0 warnings, 0 hints (32 files), after cleanup.
- `bun run lint` (biome, ts) -> clean, 53 files.
- `docker build` -> image built (Bun install + Astro build inside container OK).
- Container run on :8099 -> EN / = 200, FR /fr/ = 200, unknown = 404.
- Headers: HTML `Cache-Control: no-cache`; `/_astro/*` `public, max-age=31536000, immutable`.
- Titles correct: EN "FlowFlow - A word away", FR "FlowFlow - À portée de voix".
- Test container + test image removed; no leftover docker artifacts.

## Findings / oversights caught and fixed mid-run

- Initial nginx referenced `/404.html`, which Astro does NOT emit (no custom 404 page). Fixed to `=404`.
- Removed duplicate `Cache-Control` (expires + add_header) -> single clean header.
- Confirmed Astro "directory" build format: `index.html`, `fr/index.html`, `_astro/`, `fonts/`.

## Checkpoints

(none - no ALWAYS-ASK hazards; zero-importer dead export + additive files, all reversible)
