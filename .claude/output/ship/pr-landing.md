## FlowFlow landing site

Port the validated mockup (`landing-page/reference/draft.html`) into a
modular Astro app, ready to ship and to extend with a backend later.

### Stack
- Astro + Bun + TypeScript (strict, `noUncheckedIndexedAccess`)
- Tailwind CSS v4
- Biome (formatter + linter, lineWidth 80)
- i18n: EN default (`/`) + FR (`/fr/`), one namespace per section

### Highlights
- Clash Grotesk + Instrument Serif, self-hosted (zero CDN at runtime)
- Strict SRP: one component per section (hero, marquee, horizontal
  showcase, agentic chapter, bento, FAQ, finale, footer)
- Responsive nav: EN/FR switcher on the far right + mobile hamburger menu
- Logo served via `astro:assets`
- Green gate: `astro check` 0/0/0, Biome clean, no code line over 80

### Pending (owner)
- Capture real screenshots/GIFs for the dashed placeholder slots
  (Mac window 16:9.4, native iPhone frames, the 4 showcase GIFs)
- Future: Axum backend for the email capture + licences/Stripe

### Notes
- The site lives in `landing-page/` as its own Astro project; the Rust app
  is untouched.
- `landing-page/reference/` keeps the original mockup as the design source
  of truth.
