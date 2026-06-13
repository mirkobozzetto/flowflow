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

## Layout

- `src/components/` one component per file, composed by `Landing.astro` in draft order.
- `src/i18n/{en,fr}/` per-section JSON namespaces, plus `ui.ts` for page meta.
- `src/lib/i18n.ts` translation helpers. `src/lib/github.ts` build-time star count.
- `src/styles/` tokens, base, animations, global entry. `src/scripts/` client behaviors.
- `src/CONTRACT.md` component props, token names, i18n usage.
- `reference/draft.html` validated visual source of truth. `reference/typo-specimens.html` font study.

Fonts are self-hosted in `public/fonts/`, no runtime CDN.
