# FlowFlow web

Marketing site for FlowFlow, the 100% Rust voice-notes app. Astro + Bun + TypeScript + Tailwind v4, EN/FR.

## Develop

```bash
bun install
bun run dev
```

EN at `http://localhost:4321/`, FR at `http://localhost:4321/fr/`.

## Build

```bash
bun run build
bun run preview
bun run check
```

Static output in `dist/`.

## Deploy (Docker / Dokploy)

The site builds to static files served by nginx. `Dockerfile` is a two-stage build: Bun compiles `dist/`, nginx serves it on port 80.

Local check:

```bash
docker compose up --build
```

Then open `http://localhost:8080/` (EN) and `http://localhost:8080/fr/` (FR).

On Dokploy:

- Create an Application, point it at this repo.
- Build type: `Dockerfile`. Build path / context: `landing-page`. Dockerfile path: `landing-page/Dockerfile` (or `Dockerfile` if the app root is set to `landing-page`).
- Container port: `80`. Generate a domain and map it to port 80.
- Deploy. nginx serves the prebuilt static site; no env vars are required at runtime.

## Layout

- `src/components/` one component per file, composed by `Landing.astro` in draft order.
- `src/i18n/{en,fr}/` per-section JSON namespaces, plus `ui.ts` for page meta.
- `src/lib/i18n.ts` translation helpers. `src/lib/github.ts` build-time star count.
- `src/styles/` tokens, base, animations, global entry. `src/scripts/` client behaviors.
- `src/CONTRACT.md` component props, token names, i18n usage.
- `reference/draft.html` validated visual source of truth. `reference/typo-specimens.html` font study.

Fonts are self-hosted in `public/fonts/`, no runtime CDN.
