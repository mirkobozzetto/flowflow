import { expect, type Page, test } from "@playwright/test";

const PAGES = [
  {
    path: "/",
    h1: "A word away",
    sub: "From idea to action, no keyboard",
    heroLine:
      "Email, quote, meeting or anything else: your agent prepares it all, you just say go",
    heroGo: "you just say go",
    steps: ["Speak", "Transcribe", "Organize", "Recall"],
    stepTitles: [
      "One tap, and you talk",
      "Your meeting turns into minutes",
      "You never file anything again",
      "Ask, and it comes back",
    ],
    agentic: "Your agent",
    agenticSub:
      "Dictate a request and your Hermes agent handles it: emails, quotes, research. Nothing goes out without your OK on the iPhone.",
    teams: "You, your team, your agent",
    teamsSub:
      "Invite your team and your Hermes agent into a space: they see its notes, nothing else. You remove access whenever you like.",
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
    keys: [
      "New note",
      "New chat",
      "Search notes",
      "Theme picker",
      "Previous view",
      "Next view",
    ],
    airplane: "Airplane mode",
    packet: "Note",
    appBadge: "Download on the App Store",
    macBadge: "Download for macOS",
    unsureTitle: "Tailored support",
    unsureText:
      "30 minutes to scope your needs. We shape the offer together, and I stay on as your support.",
    notes: [
      "nobody in between",
      "even in airplane mode",
      "the provider's price, nothing on top",
    ],
  },
  {
    path: "/fr/",
    h1: "À portée de voix",
    sub: "De l'idée à l'action, sans clavier",
    heroLine:
      "Mail, devis, rendez-vous ou autre chose : ton agent prépare tout, tu n'as qu'à dire go",
    heroGo: "tu n'as qu'à dire go",
    steps: ["Parle", "Transcrit", "Range", "Retrouve"],
    stepTitles: [
      "Un tap, et tu parles",
      "Ta réunion devient un compte rendu",
      "Tu ne classes plus rien",
      "Demande, tu retrouves",
    ],
    agentic: "Ton agent",
    agenticSub:
      "Tu dictes une demande, ton agent Hermes s'en occupe : mails, devis, veille. Rien ne part sans ton accord sur l'iPhone.",
    teams: "Toi, ton équipe, ton agent",
    teamsSub:
      "Invite ton équipe et ton agent Hermes dans un espace : ils voient ses notes, rien d'autre. Tu retires l'accès quand tu veux.",
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
    keys: [
      "Nouvelle note",
      "Nouveau chat",
      "Rechercher dans les notes",
      "Sélecteur de thèmes",
      "Vue précédente",
      "Vue suivante",
    ],
    airplane: "Mode avion",
    packet: "Note",
    appBadge: "Télécharger dans l'App Store",
    macBadge: "Télécharger pour macOS",
    unsureTitle: "Un accompagnement sur mesure",
    unsureText:
      "30 minutes pour cerner ton besoin. L'offre se bâtit avec toi, et je reste en soutien.",
    notes: [
      "personne au milieu",
      "même en mode avion",
      "le prix du fournisseur, rien de plus",
    ],
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
const EMAIL_PATTERN = /[\w.+-]+@[\w-]+\.[a-z]{2,}/gi;
const BOOKING_LINKS = 2;
const EMAIL_FREE_PAGES = [
  "/",
  "/fr/",
  "/hermes",
  "/fr/hermes",
  "/hermes/espaces",
  "/beta",
  "/fr/beta",
];
const CLIP_RATIO = 720 / 1564;
const MAX_CLIP_CROP = 0.01;
const HERMES_WAVE_GAP = 56;
const EDGE_TOLERANCE_PX = 2;
const TAILSCALE_WHITE = "/logos/tailscale-white.svg";
const MIN_INK_ICONS = 6;
const CENTER_TOLERANCE_PX = 1.5;
const MIN_SPOKEN_TITLE_LEADING = 1.15;
const HERO_MASCOT_PX = 176;
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
      await expect(page.locator("header p").first()).toHaveText(copy.sub);
      await expect(page.locator("header .hero-line")).toHaveText(copy.heroLine);
      await expect(page.locator("header .hero-line mark.hl")).toHaveText(
        copy.heroGo,
      );
      const subSize = await page
        .locator("header p")
        .first()
        .evaluate((e) => Number.parseFloat(getComputedStyle(e).fontSize));
      expect(subSize).toBeGreaterThanOrEqual(MIN_HERO_SUB_PX);
      await expect(page.locator("#product .step")).toHaveText(copy.steps);
      await expect(page.locator("#product h3")).toHaveText(copy.stepTitles);
      await expect(page.locator("#agentic h2")).toContainText(copy.agentic);
      await expect(page.locator("#agentic .sub")).toHaveText(copy.agenticSub);
      await expect(page.locator("#teams h2")).toContainText(copy.teams);
      await expect(page.locator("#teams h2 + p")).toHaveText(copy.teamsSub);
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

      const wave = page.locator(".capsule canvas");
      await expect(wave).toHaveAttribute("data-wave", "live");
      const level = await wave.getAttribute("data-level");
      await expect.poll(() => wave.getAttribute("data-level")).not.toBe(level);
      const mask = await page
        .locator(".mac-screen .prompter")
        .evaluate((e) => getComputedStyle(e).maskImage);
      expect(mask).not.toMatch(
        /^linear-gradient\((rgba\(0, 0, 0, 0\)|transparent)/,
      );
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
      // No network-idle wait: looping videos keep the network busy.
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
        const badges = page.locator(`${zone} .hero-ctas a`);
        await expect(badges).toHaveText([copy.appBadge, copy.macBadge]);
        await expect(badges.nth(0)).toHaveAttribute("href", /apps\.apple\.com/);
        await expect(badges.nth(1)).toHaveAttribute("href", /\.dmg$/);
        await expect(badges.nth(0).locator("svg")).toHaveCount(1);
        await expect(badges.nth(1).locator("svg.macbook")).toHaveCount(1);
        expect(
          await badges
            .nth(1)
            .evaluate((e) => getComputedStyle(e).backgroundColor),
        ).toBe(WHITE);
        const [app, mac] = await Promise.all([
          badges.nth(0).boundingBox(),
          badges.nth(1).boundingBox(),
        ]);
        expect(Math.abs((app?.width ?? 0) - (mac?.width ?? 1))).toBeLessThan(1);
        expect(Math.abs((app?.height ?? 0) - (mac?.height ?? 1))).toBeLessThan(
          1,
        );
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

    test("keeps every phone screen in its clip's shape", async ({ page }) => {
      const ratios = await page
        .locator("#product .phone-screen, #agentic .phone-screen")
        .evaluateAll((screens) =>
          screens.map((s) => s.clientWidth / s.clientHeight),
        );
      expect(ratios.length).toBeGreaterThan(0);
      for (const ratio of ratios) {
        expect(Math.abs(ratio / CLIP_RATIO - 1)).toBeLessThan(MAX_CLIP_CROP);
      }
    });

    test("hides the page behind a solid menu on a phone", async ({
      page,
      isMobile,
    }) => {
      test.skip(!isMobile, "the band only covers the phone menu");
      const fill = await page
        .locator("nav .nav-inner")
        .evaluate((e) => getComputedStyle(e).backgroundColor);
      expect(fill.startsWith("rgba")).toBe(false);
      const band = await page.locator("nav").evaluate((nav) => {
        const inner = nav.querySelector(".nav-inner") as HTMLElement;
        const before = getComputedStyle(nav, "::before");
        return {
          content: before.content,
          height: Number.parseFloat(before.height),
          cover: inner.offsetTop + inner.offsetHeight,
        };
      });
      expect(band.content).not.toBe("none");
      expect(band.height).toBeGreaterThanOrEqual(band.cover);
    });

    test("sets Tailscale white on a black tile", async ({ page }) => {
      const mark = page.locator("#agentic .hermes img.tailscale");
      await expect(mark).toHaveAttribute("src", TAILSCALE_WHITE);
      expect(
        await mark.evaluate((e) => getComputedStyle(e).backgroundColor),
      ).toBe(BLACK);
    });

    test("leaves air between the Hermes card and the wave", async ({
      page,
    }) => {
      const gap = await page.locator("#agentic").evaluate((section) => {
        const card = section.querySelector(".hermes") as HTMLElement;
        const wave = section.querySelector(".bars") as HTMLElement;
        return (
          wave.getBoundingClientRect().top - card.getBoundingClientRect().bottom
        );
      });
      expect(gap).toBeGreaterThanOrEqual(HERMES_WAVE_GAP);
    });

    test("passes a note between an iPhone and a MacBook", async ({ page }) => {
      const sync = page.locator(".sync-visual");
      await expect(sync.locator(".iphone-art")).toHaveCount(1);
      await expect(sync.locator(".macbook-art")).toHaveCount(1);
      await expect(sync.locator("[data-packet]")).toHaveAttribute(
        "aria-label",
        copy.packet,
      );
    });

    test("plays each shortcut in a small Mac window", async ({ page }) => {
      const card = page.locator("[data-shortcuts]");
      await card.scrollIntoViewIfNeeded();
      await expect(card.locator("kbd")).toHaveCount(copy.keys.length);
      const label = card.locator("[data-shortcut-label]");
      await expect(label).toHaveText(copy.keys[0] ?? "");
      await expect(card.locator("kbd.on")).toHaveCount(1);
      await expect(label).toHaveText(copy.keys[1] ?? "");
    });

    test("types the dictated sentence in airplane mode", async ({ page }) => {
      const card = page.locator("[data-offline]");
      await card.scrollIntoViewIfNeeded();
      await expect(card.locator(".air-row")).toContainText(copy.airplane);
      await expect
        .poll(() => card.locator("[data-typed] span.on").count())
        .toBeGreaterThan(1);
    });

    test("ends the day's cards with the same margin on both sides", async ({
      page,
      isMobile,
    }) => {
      test.skip(isMobile, "the cards stack on a phone");
      const end = await page.evaluate(() => {
        const s = document.getElementById("hscroll") as HTMLElement;
        return s.offsetTop + s.offsetHeight - innerHeight;
      });
      await page.evaluate((y) => scrollTo(0, y), end);
      await page.waitForTimeout(400);
      const margins = await page.evaluate(() => {
        const track = document.getElementById("htrack") as HTMLElement;
        const last = track.lastElementChild as HTMLElement;
        return {
          left: Number.parseFloat(getComputedStyle(track).paddingLeft),
          right:
            document.documentElement.clientWidth -
            last.getBoundingClientRect().right,
        };
      });
      expect(Math.abs(margins.right - margins.left)).toBeLessThanOrEqual(
        EDGE_TOLERANCE_PX,
      );
    });

    test("peeks the mascot from the screen edge on a wide screen", async ({
      page,
      isMobile,
    }) => {
      const mascot = page.locator(".edge-mascot");
      if (isMobile) {
        await expect(mascot).toBeHidden();
        return;
      }
      // It never stops moving, so no wait for a stable element.
      await mascot.evaluate((e) => e.scrollIntoView({ block: "center" }));
      await expect(mascot).toBeVisible();
      const box = await mascot.boundingBox();
      const width = await page.evaluate(
        () => document.documentElement.clientWidth,
      );
      expect((box?.x ?? 0) + (box?.width ?? 0)).toBeGreaterThan(width);
      expect(box?.x ?? width).toBeLessThan(width);
    });

    test("explains the tailored support under the offers", async ({ page }) => {
      const unsure = page.locator("#teams .unsure");
      await expect(unsure.locator(".unsure-title")).toHaveText(
        copy.unsureTitle,
      );
      await expect(unsure.locator(".unsure-text")).toHaveText(copy.unsureText);
      await expect(unsure.locator("svg.arrow-up")).toHaveCount(1);
    });

    test("puts my photo on the guided offer and the mascot on the solo one", async ({
      page,
    }) => {
      const faces = page.locator(
        '#teams .offer[data-tone="orange"] .faces img',
      );
      await expect(faces).toHaveCount(1);
      await expect(
        page.locator('#teams .offer[data-tone="white"] .offer-mascot'),
      ).toHaveCount(1);
    });

    test("encrypts the note between a real iPhone and a MacBook", async ({
      page,
    }) => {
      const sync = page.locator(".sync-visual");
      await expect(sync.locator(".iphone-art .island")).toHaveCount(1);
      await expect(sync.locator(".macbook-art .notch")).toHaveCount(1);
      const offset = await sync.evaluate((root) => {
        const line = root.querySelector(".sync-line") as HTMLElement;
        const lock = root.querySelector(".sync-lock") as HTMLElement;
        const a = line.getBoundingClientRect();
        const b = lock.getBoundingClientRect();
        return {
          x: Math.abs(a.left + a.width / 2 - (b.left + b.width / 2)),
          y: Math.abs(a.top + a.height / 2 - (b.top + b.height / 2)),
        };
      });
      expect(offset.x).toBeLessThanOrEqual(CENTER_TOLERANCE_PX);
      expect(offset.y).toBeLessThanOrEqual(CENTER_TOLERANCE_PX);
      await expect(sync.locator("[data-packet] svg")).toHaveCount(1);
    });

    test("draws the iPhone and the MacBook next to the local promise", async ({
      page,
    }) => {
      const stay = page.locator("#trust .stay");
      await expect(stay.locator(".iphone-art .island")).toHaveCount(1);
      await expect(stay.locator(".macbook-art .notch")).toHaveCount(1);
    });

    test("scatters app icons, a dot grid and hand notes on a wide screen", async ({
      page,
      isMobile,
    }) => {
      const icons = page.locator(".ink-icon");
      expect(await icons.count()).toBeGreaterThanOrEqual(MIN_INK_ICONS);
      await expect(page.locator(".dot-grid").first()).toBeAttached();
      await expect(page.locator(".hand-note")).toHaveText(copy.notes);
      if (isMobile) await expect(icons.first()).toBeHidden();
    });

    test("gives the orange title blocks room above and below", async ({
      page,
    }) => {
      const leadings = await page
        .locator("h2:has(em.say)")
        .evaluateAll((titles) =>
          titles.map((t) => {
            const s = getComputedStyle(t);
            return (
              Number.parseFloat(s.lineHeight) / Number.parseFloat(s.fontSize)
            );
          }),
        );
      expect(leadings.length).toBeGreaterThan(0);
      for (const leading of leadings) {
        expect(leading).toBeGreaterThanOrEqual(MIN_SPOKEN_TITLE_LEADING);
      }
    });

    test("peeks the mascot from the left edge at the finale", async ({
      page,
      isMobile,
    }) => {
      const mascot = page.locator("#finale .edge-mascot-left");
      if (isMobile) {
        await expect(mascot).toBeHidden();
        return;
      }
      await mascot.evaluate((e) => e.scrollIntoView({ block: "center" }));
      const box = await mascot.boundingBox();
      expect(await mascot.evaluate((e) => (e as HTMLElement).offsetWidth)).toBe(
        HERO_MASCOT_PX,
      );
      expect(box?.x ?? 0).toBeLessThan(0);
      expect((box?.x ?? 0) + (box?.width ?? 0)).toBeGreaterThan(0);
    });

    test("centers the photo and the Hermes marks on a phone", async ({
      page,
      isMobile,
    }) => {
      test.skip(!isMobile, "only the phone layout stacks them");
      const offCenter = (selector: string, parent: string) =>
        page.evaluate(
          ([child, box]) => {
            const c = document.querySelector(child as string) as HTMLElement;
            const p = document.querySelector(box as string) as HTMLElement;
            const a = c.getBoundingClientRect();
            const b = p.getBoundingClientRect();
            return Math.abs(a.left + a.width / 2 - (b.left + b.width / 2));
          },
          [selector, parent],
        );
      expect(
        await offCenter("#teams .faces", '#teams .offer[data-tone="orange"]'),
      ).toBeLessThanOrEqual(CENTER_TOLERANCE_PX);
      expect(
        await offCenter("#agentic .hermes .marks", "#agentic .hermes"),
      ).toBeLessThanOrEqual(CENTER_TOLERANCE_PX);
    });

    test("shows the whole chat phone in its card on a phone", async ({
      page,
      isMobile,
    }) => {
      test.skip(!isMobile, "the desktop card crops the phone on purpose");
      const fits = await page.locator(".shots .front").evaluate((phone) => {
        const card = phone.closest(".fine") as HTMLElement;
        const a = phone.getBoundingClientRect();
        const b = card.getBoundingClientRect();
        return a.bottom <= b.bottom && a.top >= b.top;
      });
      expect(fits).toBe(true);
    });

    test("keeps the hero line on one line on a wide screen", async ({
      page,
      isMobile,
    }) => {
      test.skip(isMobile, "the line wraps on a phone");
      const lines = await page.locator("header .hero-line").evaluate((e) => {
        const s = getComputedStyle(e);
        return (
          e.getBoundingClientRect().height / Number.parseFloat(s.lineHeight)
        );
      });
      expect(Math.round(lines)).toBe(1);
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

for (const path of EMAIL_FREE_PAGES) {
  test(`keeps every email address out of ${path}`, async ({ page }) => {
    await page.goto(path);
    const html = await page.content();
    expect(html.match(EMAIL_PATTERN) ?? []).toEqual([]);
  });
}

test("books a call through an address rebuilt only on click", async ({
  page,
}) => {
  await page.goto("/");
  const book = page.locator("[data-contact]");
  expect(await book.count()).toBeGreaterThanOrEqual(BOOKING_LINKS);
  await expect(book.first()).not.toHaveAttribute("href", /mailto/);
  const target = await book
    .first()
    .evaluate((link: HTMLElement) =>
      [...atob(link.dataset.contact ?? "")].reverse().join(""),
    );
  expect(target).toMatch(/^mailto:[^@\s]+@[^@\s]+\?subject=/);
});
