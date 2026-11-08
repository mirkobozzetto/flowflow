import { expect, type Page, test } from "@playwright/test";

const PAGES = [
  {
    path: "/",
    h1: "A word away",
    sub: "You dictate, FlowFlow transcribes, files and takes action.",
    steps: ["Speak", "Transcribe", "Organize", "Recall"],
    stepTitles: [
      "One tap, and you talk",
      "Your meeting turns into minutes",
      "You never file anything again",
      "Ask, and it comes back",
    ],
    agentic: "From word",
    teams: "You, your team, your agent",
    trust: "You always know",
    free: "Free to start",
    firstPrice: "€0",
    book: "Book a call",
    calendar: "Google Calendar",
    soon: "Coming next",
    hermes: "Link my Hermes",
    tailscale: "Tailscale recommended",
    hermesPath: "/hermes",
    stepProofs: ["each with its timestamp", "cites its notes"],
    trustProofs: ["only when you use it", "with no markup"],
    ai: "Codex/ChatGPT and Claude",
    aiUse: "your subscription",
  },
  {
    path: "/fr/",
    h1: "À portée de voix",
    sub: "Tu dictes, FlowFlow transcrit, range et passe à l'action.",
    steps: ["Parle", "Transcrit", "Range", "Retrouve"],
    stepTitles: [
      "Un tap, et tu parles",
      "Ta réunion devient un compte rendu",
      "Tu ne classes plus rien",
      "Demande, tu retrouves",
    ],
    agentic: "Du dire",
    teams: "Toi, ton équipe, ton agent",
    trust: "Tu sais toujours",
    free: "Gratuit pour commencer",
    firstPrice: "0\u00a0€",
    book: "Prendre rendez-vous",
    calendar: "Google Agenda",
    soon: "À venir",
    hermes: "Relier mon Hermes",
    tailscale: "Tailscale conseillé",
    hermesPath: "/fr/hermes",
    stepProofs: ["chacun à sa minute", "cite ses notes"],
    trustProofs: ["seulement quand tu t'en sers", "sans marge"],
    ai: "Codex/ChatGPT et Claude",
    aiUse: "ton abonnement",
  },
];

const FORBIDDEN = ["REAL GIF", "REAL SCREENSHOT", "GIF RÉEL", "100% Rust"];
const SOON = /\b(soon|bientôt)\b/i;
const WHITE = "rgb(255, 255, 255)";
const BLACK = "rgb(0, 0, 0)";
const MIN_CLIPS = 7;
const CONNECTORS = 9;
const SOON_TOOLS = 12;
const SOFT_EDGE = "rgba(0, 0, 0, 0.16)";
const CAPSULE_BLACK = "rgb(20, 20, 20)";
const ACCENT = "rgb(255, 92, 0)";
const CARD_WEIGHT = "600";
const MIN_SECTION_WAVES = 2;
const MAX_PROMO_CROP = 0.01;
const MIN_HERO_PHONE_WIDTH = 280;
const LEAVE_ROWS = 4;
const LEAVE_LOGOS = 5;
const HERMES_MARKS = 2;
const PRICE_CARDS = 3;
const HARD_RING = "rgba(0, 0, 0, 0.28)";
const MASCOT = "/brand/mascot.webp";
const BODY_WEIGHT = "500";
const MIN_STEP_TEXT_PX = 16;
const MIN_NAV_OPACITY = 0.9;
const MIN_HERO_SUB_PX = 20;
const MOCK_STARS = "321";
const MIN_PROMPTER_LINES = 20;
const RUST_URL = "https://www.rust-lang.org";
const DIOXUS_URL = "https://dioxuslabs.com";

async function scrollThrough(page: Page): Promise<void> {
  await page.evaluate(async () => {
    const step = innerHeight * 0.8;
    for (let y = 0; y < document.body.scrollHeight; y += step) {
      scrollTo(0, y);
      await new Promise((done) => setTimeout(done, 60));
    }
    scrollTo(0, 0);
  });
}

const rowLengths = (page: Page, selector: string): Promise<number[]> =>
  page.locator(selector).evaluateAll((items) => {
    const rows = new Map<number, number>();
    for (const item of items) {
      const top = Math.round(item.getBoundingClientRect().top);
      rows.set(top, (rows.get(top) ?? 0) + 1);
    }
    return [...rows.values()];
  });

const colorOf = (page: Page, selector: string): Promise<string> =>
  page
    .locator(selector)
    .first()
    .evaluate((element) => getComputedStyle(element).color);

for (const copy of PAGES) {
  test.describe(copy.path, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(copy.path);
    });

    test("keeps the headline and shows the exact copy", async ({ page }) => {
      await expect(page.locator("h1")).toHaveText(copy.h1);
      await expect(page.locator("header p").first()).toContainText(copy.sub);
      const subSize = await page
        .locator("header p")
        .first()
        .evaluate((e) => Number.parseFloat(getComputedStyle(e).fontSize));
      expect(subSize).toBeGreaterThanOrEqual(MIN_HERO_SUB_PX);
      await expect(page.locator("#product .step")).toHaveText(copy.steps);
      await expect(page.locator("#product h3")).toHaveText(copy.stepTitles);
      await expect(page.locator("#agentic h2")).toContainText(copy.agentic);
      await expect(page.locator("#teams h2")).toContainText(copy.teams);
      await expect(page.locator("#trust h2")).toContainText(copy.trust);
      await expect(page.locator("#trust .cost h3")).toHaveText(copy.free);
    });

    test("has no empty frame, form or soon badge", async ({ page }) => {
      const text = (await page.locator("body").innerText()) ?? "";
      for (const word of FORBIDDEN) expect(text).not.toContain(word);
      expect(text).not.toMatch(SOON);
      await expect(page.locator("form")).toHaveCount(0);
      await expect(page.locator(".marquee")).toHaveCount(0);
    });

    test("shows the real connectors", async ({ page }) => {
      const chips = page.locator("#agentic .chips li");
      await expect(chips).toHaveCount(CONNECTORS);
      await expect(chips.nth(2)).toContainText(copy.calendar);
      await expect(
        page.locator("#agentic .chips li").locator("svg, img"),
      ).toHaveCount(CONNECTORS);
      await expect(chips.last()).toContainText("GitHub");
    });

    test("lays the connectors out with no orphan", async ({
      page,
      isMobile,
    }) => {
      const rows = await rowLengths(page, "#agentic .chips li");
      expect(rows).toEqual(isMobile ? [2, 2, 2, 2, 1] : [3, 3, 3]);
      const [last, grid] = await Promise.all([
        page.locator("#agentic .chips li").last().boundingBox(),
        page.locator("#agentic .chips").boundingBox(),
      ]);
      expect(last?.width).toBeCloseTo(
        isMobile ? (grid?.width ?? 0) : (last?.width ?? 0),
        0,
      );
    });

    test("lists the coming tools under their own label", async ({ page }) => {
      await expect(page.locator("#agentic .soon-label")).toHaveText(copy.soon);
      const tools = page.locator("#agentic .soon li");
      await expect(tools).toHaveCount(SOON_TOOLS);
      await expect(tools.nth(1)).toContainText("Slack");
    });

    test("gives white blocks and buttons a light gradient and a crisp edge", async ({
      page,
    }) => {
      for (const selector of [
        ".fine",
        '.offer[data-tone="white"]',
        "#trust .prices li",
        ".btn-ghost",
        "#agentic .chips li",
        "#agentic .soon li",
        "footer .paper-chip",
      ]) {
        const surface = page.locator(selector).first();
        const style = await surface.evaluate((e) => {
          const s = getComputedStyle(e);
          return { image: s.backgroundImage, shadow: s.boxShadow };
        });
        expect(style.image).toContain("linear-gradient");
        expect(style.shadow).toContain("inset");
        expect(style.shadow).toContain(SOFT_EDGE);
      }
    });

    test("keeps the white button on orange plain white", async ({ page }) => {
      const button = page.locator(".btn-white").first();
      expect(
        await button.evaluate((e) => getComputedStyle(e).backgroundImage),
      ).toBe("none");
      expect(
        await button.evaluate((e) => getComputedStyle(e).backgroundColor),
      ).toBe(WHITE);
    });

    test("keeps a white button soft on hover", async ({ page, isMobile }) => {
      test.skip(isMobile, "no hover on touch");
      const button = page.locator(".btn-ghost").first();
      await button.hover();
      await page.waitForTimeout(300);
      const style = await button.evaluate((e) => {
        const s = getComputedStyle(e);
        return {
          image: s.backgroundImage,
          filter: s.filter,
          shadow: s.boxShadow,
        };
      });
      expect(style.image).toContain("linear-gradient");
      expect(style.filter).toBe("none");
      expect(style.shadow).not.toContain(HARD_RING);
    });

    test("does not lift a button on hover", async ({ page, isMobile }) => {
      test.skip(isMobile, "no hover on touch");
      const button = page.locator(".btn-primary").first();
      await button.hover();
      await page.waitForTimeout(250);
      expect(await button.evaluate((e) => getComputedStyle(e).transform)).toBe(
        "none",
      );
    });

    test("sets the action cards straight and semi-bold", async ({ page }) => {
      const you = page.locator("#agentic .action .you").first();
      expect(await you.evaluate((e) => getComputedStyle(e).fontStyle)).toBe(
        "normal",
      );
      expect(await you.evaluate((e) => getComputedStyle(e).fontWeight)).toBe(
        CARD_WEIGHT,
      );
    });

    test("keeps the action cards free of icons", async ({ page }) => {
      await expect(page.locator("#agentic .action svg")).toHaveCount(0);
    });

    test("gives every coming tool its logo", async ({ page }) => {
      await expect(page.locator("#agentic .soon li img")).toHaveCount(
        SOON_TOOLS,
      );
    });

    test("puts the outside services forward with their logos", async ({
      page,
    }) => {
      const rows = page.locator("#trust .leave li");
      await expect(rows).toHaveCount(LEAVE_ROWS);
      await expect(rows.locator(".logos img")).toHaveCount(LEAVE_LOGOS);
      await expect(rows.nth(1).locator(".who")).toHaveText("Soniox");
      await expect(rows.nth(0).locator(".who")).toHaveText(copy.ai);
      await expect(rows.nth(0).locator(".use")).toContainText(copy.aiUse);
      await expect(rows.nth(0).locator("img[src$='claude.svg']")).toHaveCount(
        1,
      );
      const rings = await rows
        .locator(".logos img")
        .evaluateAll((logos) =>
          logos.map((l) => getComputedStyle(l).boxShadow),
        );
      for (const ring of rings) expect(ring).toBe("none");
      const lefts = await rows
        .locator(".who")
        .evaluateAll((names) =>
          names.map((name) => Math.round(name.getBoundingClientRect().left)),
        );
      expect(new Set(lefts).size).toBe(1);
    });

    test("keeps the privacy card white with a tick on each local point", async ({
      page,
    }) => {
      const leave = page.locator("#trust .leave");
      expect(
        await leave.evaluate((e) => getComputedStyle(e).backgroundImage),
      ).toBe("none");
      const ticks = await page
        .locator("#trust .stay li")
        .evaluateAll((items) =>
          items.map((item) => getComputedStyle(item, "::before").content),
        );
      expect(ticks.length).toBeGreaterThan(0);
      for (const tick of ticks) expect(tick).not.toBe("none");
    });

    test("puts Hermes and Tailscale forward with a strong button", async ({
      page,
    }) => {
      const card = page.locator("#agentic .hermes");
      await expect(card.locator("img")).toHaveCount(HERMES_MARKS);
      await expect(card.locator("a.btn-primary")).toHaveText(copy.hermes);
      await expect(card.locator(".note")).toContainText(copy.tailscale);
      await expect(card.locator("a.btn-primary")).toHaveAttribute(
        "href",
        copy.hermesPath,
      );
    });

    test("sets body text in medium and step titles in semi-bold", async ({
      page,
    }) => {
      expect(
        await page
          .locator("body")
          .evaluate((e) => getComputedStyle(e).fontWeight),
      ).toBe(BODY_WEIGHT);
      const title = page.locator("#product h3").first();
      expect(await title.evaluate((e) => getComputedStyle(e).fontWeight)).toBe(
        CARD_WEIGHT,
      );
      const text = page.locator("#product article p").first();
      expect(
        await text.evaluate((e) =>
          Number.parseFloat(getComputedStyle(e).fontSize),
        ),
      ).toBeGreaterThanOrEqual(MIN_STEP_TEXT_PX);
    });

    test("backs the first two steps with facts", async ({ page }) => {
      const steps = page.locator("#product article");
      await expect(steps.nth(0).locator(".facts li")).toHaveCount(2);
      await expect(steps.nth(1).locator(".facts li")).toHaveCount(3);
    });

    test("shows the price in three cards and a booking link", async ({
      page,
    }) => {
      const cards = page.locator("#trust .prices li");
      await expect(cards).toHaveCount(PRICE_CARDS);
      await expect(cards.first().locator(".price")).toHaveText(copy.firstPrice);
      await expect(cards.last().locator("img[src$='claude.svg']")).toHaveCount(
        1,
      );
      await expect(page.locator("#trust .cost a")).toHaveText(copy.book);
    });

    test("draws the voice waves", async ({ page }) => {
      await expect(page.locator("#agentic > canvas")).toHaveAttribute(
        "data-wave",
        "bars",
      );
      const rules = await page
        .locator(".wave-rule canvas[data-wave='bars']")
        .count();
      expect(rules).toBeGreaterThanOrEqual(MIN_SECTION_WAVES);
    });

    test("shows the black capsule, one word at a time", async ({ page }) => {
      const capsule = page.locator(".capsule");
      await expect(capsule).toBeVisible();
      expect(
        await capsule.evaluate((e) => getComputedStyle(e).backgroundColor),
      ).toBe(CAPSULE_BLACK);

      const word = page.locator("[data-spoken] span.now");
      await expect(word).toHaveCount(1);
      const first = await word.textContent();
      await expect.poll(() => word.textContent()).not.toBe(first);
      expect(
        await word.evaluate((e) => getComputedStyle(e).backgroundColor),
      ).toBe(ACCENT);
      expect(await word.evaluate((e) => getComputedStyle(e).color)).toBe(WHITE);
    });

    test("scrolls a long day of lines in the Mac window", async ({ page }) => {
      const lines = page.locator(".mac-screen [data-spoken] p");
      expect(await lines.count()).toBeGreaterThanOrEqual(MIN_PROMPTER_LINES);
      await expect(page.locator(".mac-screen img")).toHaveCount(0);
      const column = page.locator(".mac-screen [data-spoken]");
      const start = await column.evaluate((e) => getComputedStyle(e).transform);
      await expect
        .poll(() => column.evaluate((e) => getComputedStyle(e).transform), {
          timeout: 15000,
        })
        .not.toBe(start);
    });

    test("has no scroll progress bar", async ({ page }) => {
      await expect(page.locator("#progress")).toHaveCount(0);
    });

    test("uses white on orange and black on white", async ({ page }) => {
      expect(await colorOf(page, "#agentic h2")).toBe(WHITE);
      expect(await colorOf(page, "#agentic h2 em")).toBe(ACCENT);
      expect(await colorOf(page, "#agentic .sub")).toBe(WHITE);
      expect(await colorOf(page, ".btn-primary")).toBe(WHITE);
      expect(await colorOf(page, '.offer[data-tone="orange"] h3')).toBe(WHITE);
      expect(await colorOf(page, "#trust .leave h3")).toBe(BLACK);
      expect(await colorOf(page, '.offer[data-tone="white"] h3')).toBe(BLACK);
      expect(await colorOf(page, "#teams > p")).toBe(BLACK);
      expect(await colorOf(page, "#agentic .action .then")).toBe(BLACK);
    });

    test("loads every image", async ({ page }) => {
      await scrollThrough(page);
      await page.waitForLoadState("networkidle");
      const broken = await page.evaluate(async () => {
        const images = [...document.images];
        for (const image of images) {
          image.loading = "eager";
          await image.decode().catch(() => undefined);
        }
        return images
          .filter((image) => image.naturalWidth === 0)
          .map((image) => image.src);
      });
      expect(broken).toEqual([]);
    });

    test("every clip plays once in view", async ({ page }) => {
      const clips = page.locator("video[data-lazy]");
      const count = await clips.count();
      expect(count).toBeGreaterThanOrEqual(MIN_CLIPS);
      for (let i = 0; i < count; i += 1) {
        const clip = clips.nth(i);
        await clip.evaluate((v) => v.scrollIntoView({ block: "center" }));
        await expect
          .poll(() => clip.evaluate((v: HTMLVideoElement) => v.currentTime), {
            timeout: 8000,
          })
          .toBeGreaterThan(0.1);
        expect(await clip.evaluate((v: HTMLVideoElement) => v.muted)).toBe(
          true,
        );
      }
    });

    test("shows the whole promo in a larger hero phone", async ({ page }) => {
      const clip = page.locator(".iphone video");
      await clip.evaluate((v) => v.scrollIntoView({ block: "center" }));
      await expect
        .poll(() => clip.evaluate((v: HTMLVideoElement) => v.videoWidth))
        .toBeGreaterThan(0);
      const crop = await clip.evaluate((v: HTMLVideoElement) => {
        const frame = v.clientWidth / v.clientHeight;
        return Math.abs(frame / (v.videoWidth / v.videoHeight) - 1);
      });
      expect(crop).toBeLessThan(MAX_PROMO_CROP);
      const width = await page
        .locator(".iphone .phone")
        .evaluate((e) => (e as HTMLElement).offsetWidth);
      expect(width).toBeGreaterThanOrEqual(MIN_HERO_PHONE_WIDTH);
    });

    test("marks each title's key word as a spoken word", async ({ page }) => {
      const paper = page.locator("#agentic h2 em.say");
      const paperStyle = await paper.evaluate((e) => {
        const s = getComputedStyle(e);
        return { image: s.backgroundImage, shadow: s.boxShadow };
      });
      expect(paperStyle.image).toContain("linear-gradient");
      expect(paperStyle.shadow).toContain("inset");
      const spoken: [string, string][] = [
        ["#trust h2 em.say", ACCENT],
        ["#teams h2 em.say", ACCENT],
      ];
      for (const [selector, background] of spoken) {
        const word = page.locator(selector);
        await expect(word).toHaveCount(1);
        expect(
          await word.evaluate((e) => getComputedStyle(e).backgroundColor),
        ).toBe(background);
        expect(await word.evaluate((e) => getComputedStyle(e).fontStyle)).toBe(
          "normal",
        );
      }
      const serif = await page.evaluate(() =>
        [...document.querySelectorAll("body *")].some((e) =>
          getComputedStyle(e).fontFamily.includes("Instrument Serif"),
        ),
      );
      expect(serif).toBe(false);
    });

    test("keeps the hero word black over a lively voice wave", async ({
      page,
    }) => {
      const word = page.locator("h1 em");
      expect(await word.evaluate((e) => getComputedStyle(e).color)).toBe(BLACK);
      await expect(word.locator("canvas")).toHaveAttribute("data-wave", "bars");
      expect(
        await page
          .locator("h1")
          .evaluate((e) => getComputedStyle(e).fontWeight),
      ).toBe(CARD_WEIGHT);
      const finale = page.locator("#finale h2 em");
      expect(await finale.evaluate((e) => getComputedStyle(e).color)).toBe(
        BLACK,
      );
      await expect(finale.locator("canvas")).toHaveAttribute(
        "data-wave",
        "bars",
      );
    });

    test("links Hermes in the same tab instead of Beta", async ({ page }) => {
      const nav = page.locator("nav");
      await expect(nav.locator("a[href*='beta']")).toHaveCount(0);
      const hermes = nav.locator(`a[href='${copy.hermesPath}']`).first();
      await expect(hermes).toHaveText("Hermes");
      await expect(hermes).not.toHaveAttribute("target", "_blank");
    });

    test("keeps the nav nearly opaque over dark sections", async ({ page }) => {
      const fill = await page
        .locator("nav .nav-inner")
        .evaluate((e) => getComputedStyle(e).backgroundColor);
      const alpha = Number(fill.match(/([\d.]+)\)$/)?.[1] ?? 1);
      expect(alpha).toBeGreaterThanOrEqual(MIN_NAV_OPACITY);
    });

    test("puts the App Store first, the Mac download second", async ({
      page,
      isMobile,
    }) => {
      for (const zone of ["header", "#finale"]) {
        const buttons = page.locator(`${zone} .hero-ctas a`);
        await expect(buttons.nth(0)).toHaveClass(/btn-primary/);
        await expect(buttons.nth(0)).toHaveAttribute(
          "href",
          /apps\.apple\.com/,
        );
        await expect(buttons.nth(1)).toHaveClass(/btn-ghost/);
        await expect(buttons.nth(1)).toHaveAttribute("href", /\.dmg$/);
      }
      test.skip(isMobile, "the nav button sits in the menu on a phone");
      const navCta = page.locator("nav .nav-cta").first();
      await expect(navCta).toHaveText("App Store");
      await expect(navCta.locator("svg")).toHaveCount(1);
      await expect(navCta).toHaveAttribute("href", /apps\.apple\.com/);
    });

    test("shows the live GitHub star count", async ({ page }) => {
      await page.route("**/img.shields.io/**", (route) =>
        route.fulfill({ json: { value: MOCK_STARS, message: MOCK_STARS } }),
      );
      await page.reload();
      await expect(page.locator("footer [data-stars]")).toHaveText(MOCK_STARS);
    });

    test("ends no title with a full stop", async ({ page }) => {
      const titles = await page
        .locator("h1, h2, h3")
        .evaluateAll((all) => all.map((t) => (t.textContent ?? "").trim()));
      expect(titles.filter((t) => t.endsWith("."))).toEqual([]);
    });

    test("shows the cut-out mascot without a white tile", async ({ page }) => {
      const logo = page.locator("header img.hero-logo");
      await expect(logo).toHaveAttribute("src", MASCOT);
      expect(await logo.evaluate((e) => getComputedStyle(e).boxShadow)).toBe(
        "none",
      );
    });

    test("highlights one proof in the cards", async ({ page }) => {
      await expect(page.locator("#product .hl")).toHaveText(copy.stepProofs);
      await expect(page.locator("#trust .hl")).toHaveText(copy.trustProofs);
      await expect(page.locator("#product .num").first()).toHaveText("01");
    });

    test("does not scroll sideways", async ({ page }) => {
      await scrollThrough(page);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });
  });
}

const FOOTER_PAGES = [
  "/",
  "/fr/",
  "/hermes",
  "/fr/hermes",
  "/hermes/espaces",
  "/beta",
];

for (const path of FOOTER_PAGES) {
  test(`credits Rust and Dioxus with their logos on ${path}`, async ({
    page,
  }) => {
    await page.goto(path);
    for (const site of [RUST_URL, DIOXUS_URL]) {
      const link = page.locator(`footer a[href='${site}']`);
      await expect(link).toHaveCount(1);
      await expect(link.locator("img")).toBeVisible();
      expect(
        await link
          .locator("img")
          .evaluate((img: HTMLImageElement) => img.naturalWidth),
      ).toBeGreaterThan(0);
    }
  });
}
