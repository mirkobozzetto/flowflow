---
type: verification-bundle
slug: astro-cleanup-docker
status: shipped
---

# Verification Bundle: astro-cleanup-docker

All SAFE checks below were already run by ship and passed. Re-run any to confirm.

## 1. Code cleanup (SAFE)

```bash
cd landing-page
bun run check    # astro check -> expect 0 errors / 0 warnings / 0 hints
bun run lint     # biome on .ts -> expect clean
```

Result observed: check = 0/0/0 over 32 files; lint clean over 53 files.

Note: the landing was already clean of unused locals/imports/params (verified by injecting
`noUnusedLocals` + `noUnusedParameters` into a temp tsconfig -> astro check still 0). The only genuinely
dead symbol was the exported, never-imported `getLangFromUrl` in `src/lib/i18n.ts`; it was removed
(`noUnusedLocals` cannot flag exported-but-unused). `isLang` is kept (used internally + documented public helper).

## 2. Docker build + serve locally (SAFE)

```bash
cd landing-page
docker compose up --build        # builds image, serves on http://localhost:8080
# in another shell:
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/      # 200
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/fr/   # 200
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/nope  # 404
docker compose down
```

Result observed (ship ran the equivalent on port 8099): EN 200, FR 200, unknown 404,
HTML `no-cache`, `/_astro/*` `public, max-age=31536000, immutable`.

## 3. Deploy on the VPS via Dokploy (USER runs - involves the live VPS)

1. Dokploy -> Create Application -> connect this git repo.
2. Build type: `Dockerfile`.
3. Set the application source/root to `landing-page` (so context + Dockerfile + nginx.conf resolve),
   or keep repo root and set Dockerfile path to `landing-page/Dockerfile` with build context `landing-page`.
4. Container port: `80`.
5. Generate a domain, map it to port 80, Deploy.

No runtime env vars are required (static site; the GitHub star count is fetched at build time).

## Files changed

- `landing-page/src/lib/i18n.ts`  (removed dead `getLangFromUrl`)
- `landing-page/src/CONTRACT.md`   (doc synced)
- `landing-page/README.md`         (added Deploy / Dokploy section)
- `landing-page/Dockerfile`        (new, multi-stage Bun -> nginx)
- `landing-page/nginx.conf`        (new)
- `landing-page/.dockerignore`     (new)
- `landing-page/docker-compose.yml`(new)
