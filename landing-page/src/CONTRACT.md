# FlowFlow web - component contract

The visual source of truth is `reference/draft.html`. Section order, spacing, colors and copy all derive from it. This file documents how the pieces fit so builders integrate without guessing.

## Stack

- Astro 6 (static output), Bun runtime, TypeScript strict.
- Tailwind v4 via `@tailwindcss/vite` (no config file, no PostCSS).
- i18n: Astro built-in routing. `defaultLocale: "en"`, `locales: ["en", "fr"]`, `prefixDefaultLocale: false`. EN at `/`, FR at `/fr/`.
- Fonts self-hosted in `public/fonts/` (Clash Grotesk 400/500/600/700, Instrument Serif italic + normal). Zero runtime CDN.

## Design tokens (`src/styles/tokens.css`)

Use these CSS variables, never raw hex for brand surfaces.

- Surfaces: `--warm-white`, `--stone-50` .. `--stone-700`, `--stone-900`, `--ink`, `--ink-2`.
- Brand: `--orange`, `--orange-bright`, `--orange-dark`, `--orange-50`, `--orange-100` (all oklch).
- Type: `--sans` (Clash Grotesk, body default), `--serif` (Instrument Serif italic, accents only).

Shared primitives in `src/styles/base.css`: `.container`, `.fig`, `.reveal` (+ `[data-d="1|2|3"]`), `.btn-primary`, `.btn-ghost`, `.hero-ctas`, `#progress`, noise overlay, reduced-motion block. Keyframes in `src/styles/animations.css`. Everything is pulled together by `src/styles/global.css`, imported once in `Layout.astro`.

## i18n helpers (`src/lib/i18n.ts`)

Per-section strings live in namespaced JSON: `src/i18n/{en,fr}/{hero,showcase,agentic,bento,faq,footer}.json`. Two access patterns, both supported, pick per component:

1. `useTranslations(lang, namespace)` returns a getter `t(key)` that reads dot-paths and arrays.

   ```ts
   const t = useTranslations(lang, "hero");
   t("hero.h1Accent");
   t<QaItem[]>("items");
   ```

2. `section(lang, namespace)` returns the typed namespace object directly (best for mapping arrays of objects).

   ```ts
   const t = section(lang, "agentic");
   t.actions.map((a) => a.you);
   ```

Both fall back to EN when an FR key is missing. `localizePath(path, lang)` builds locale-prefixed hrefs (path first, lang second). `isLang(value)` is available for runtime checks. Page-level meta (title, description, og locale) lives in `src/i18n/ui.ts` via `getMeta(lang)`, consumed only by `Layout.astro`.

EN is the base locale, but the validated hooks are FR wordplay. EN JSON carries native transcreations (flagged with `_note` keys for review), not literal translations.

## Component props

Components take `lang: string` and resolve their own copy, except pure presentational ones that receive ready data.

| Component | Props | Notes |
| --- | --- | --- |
| `Layout.astro` | `lang` | html shell, head, font preload, `#progress`, global script |
| `Landing.astro` | `lang` | composes every section in draft order |
| `Nav.astro` | `lang` | reads `hero` namespace (`nav.*`) |
| `Hero.astro` | `lang` | renders `ParallaxScene` |
| `ParallaxScene.astro` | `lang` | mac + iphone mock, `initParallax("scene")` |
| `Marquee.astro` | `items: string[]` | duplicated track for the loop |
| `HorizontalShowcase.astro` | `fig: string`, `panels: Panel[]` | maps `ShowcasePanel`, sticky scroll script |
| `ShowcasePanel.astro` | `num`, `title`, `text`, `slot`, `slotNote` | one numbered step |
| `AgenticChapter.astro` | `lang`, `captureEndpoint?` | dark section, maps `ActionCard` + `ConnectorChips` + `EmailCapture` |
| `ActionCard.astro` | `you`, `then`, `soon?`, `soonLabel?`, `delay?` | voice line to result |
| `ConnectorChips.astro` | `label: string`, `connectors: {name, live}[]` | live vs soon dot |
| `EmailCapture.astro` | `title`, `blurb`, `placeholder`, `inputLabel`, `submit`, `ok`, `endpoint?` | waitlist form |
| `Bento.astro` | `lang` | four feature tiles |
| `Faq.astro` | `lang` | maps `FaqItem`, `initFaq()` |
| `FaqItem.astro` | `question`, `answer` | accordion row |
| `Finale.astro` | `lang` | closing headline + CTAs |
| `Footer.astro` | `lang` | embeds `GithubStars` |
| `GithubStars.astro` | `label: string` | build-time star count, graceful `★` fallback |

`Panel`: `{ num, title, text, slot, slotNote }`. `QaItem`: `{ q, a }`. `Connector`: `{ name, live }`.

## Client scripts (`src/scripts/`)

Pure TS, no i18n. Imported from the component that needs them.

- `progress.ts`: progress bar width + `.reveal` IntersectionObserver. Loaded once in `Layout.astro`.
- `parallax.ts`: `initParallax(sceneId)`, mouse-driven 3D tilt, skips on coarse pointer or reduced motion.
- `horizontalScroll.ts`: sticky horizontal track, falls back to a vertical stack on mobile or reduced motion (`data-fallback`).
- `faq.ts`: `initFaq()`, single-open accordion with `aria-expanded`.
- `capture.ts`: waitlist submit, optional `data-endpoint`, success swap.

## Tooling

- Formatter and linter: Biome (`biome.json` at the root), not Prettier.
  `bun run format` writes fixes, `bun run lint` checks. Run before any commit.
- Line width 80 everywhere. Code lines (`.astro` frontmatter and template,
  `.ts`, `.css`) stay under 80. Long content that cannot be split without
  changing its value (i18n JSON copy strings, the noise data-URI in base.css)
  is left as is, matching what Biome itself does.
- Type-safe strict: `tsconfig` `strict` plus `noUncheckedIndexedAccess`, no
  `any`, typed component props and i18n helper. `bunx astro check` must pass
  with zero errors and warnings.
- Biome linter and assist are disabled for `.astro` files via an override:
  Biome 2.x does not parse the Astro template, so it would flag every import
  and variable used only in markup as unused. The formatter stays on for
  `.astro`. `noImportantStyles` is off because reduced-motion overrides need
  `!important` to win the cascade.

## Rules

Zero code comments. No em-dash or en-dash, plain hyphen only. French copy with correct accents. No emojis in code. One component per file. Bun only.
