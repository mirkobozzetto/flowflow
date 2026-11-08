import { expect, type Page, test } from "@playwright/test";

const KIT = "/downloads/FlowFlow-Hermes-kit-2026-10-04.zip";
const KIT_PROMPT_FILE = "POUR-HERMES.md";
const MASCOT = "/brand/mascot.webp";
const LIGHT = 200;
const GITHUB_REPO_API = "https://api.github.com/repos/mirkobozzetto/flowflow";
const LIVE_STARS = 1234;
const LIVE_LABEL = "1.2k";

const LINK_PAGES = [
  {
    path: "/hermes",
    lang: "en",
    other: "/fr/hermes",
    hermesNav: "/hermes",
    spaces: "/hermes/espaces",
    h1: "Your Hermes in FlowFlow",
    tabs: ["Talk to Hermes", "Spaces"],
    current: "Talk to Hermes",
    h2: ["What you need", "Link your Hermes in three steps"],
    h3: [
      "Run the script",
      "Scan the code",
      "Open a conversation",
      "If FlowFlow shows",
      "Give Hermes a space of notes",
    ],
    cta: "Link my Hermes",
    manual: "Settings → Integrations → Hermes",
    proofs: ["on the Hermes machine", "tests the link", "“With Hermes”"],
  },
  {
    path: "/fr/hermes",
    lang: "fr",
    other: "/hermes",
    hermesNav: "/fr/hermes",
    spaces: "/fr/hermes/espaces",
    h1: "Ton Hermes dans FlowFlow",
    tabs: ["Parler à Hermes", "Espaces"],
    current: "Parler à Hermes",
    h2: ["Ce qu’il te faut", "Relier ton Hermes en trois étapes"],
    h3: [
      "Lance le script",
      "Scanne le code",
      "Ouvre une conversation",
      "Si FlowFlow affiche",
      "Donner un espace de notes à Hermes",
    ],
    cta: "Relier mon Hermes",
    manual: "Réglages → Intégrations → Hermes",
    proofs: ["sur la machine de Hermes", "teste la liaison", "« Avec Hermes »"],
  },
];

const SPACE_PAGES = [
  {
    path: "/hermes/espaces",
    lang: "en",
    other: "/fr/hermes/espaces",
    hermesNav: "/hermes",
    h1: "Give Hermes a space",
    tabs: ["Talk to Hermes", "Spaces"],
    current: "Spaces",
    h2: [
      "What you need",
      "Choose the space",
      "Create the access",
      "Install the kit",
      "Set up Hermes",
      "Check the connection",
      "Common questions",
      "The Hermes kit",
    ],
    copied: "Copied",
    marked: "space",
    faq: 7,
  },
  {
    path: "/fr/hermes/espaces",
    lang: "fr",
    other: "/hermes/espaces",
    hermesNav: "/fr/hermes",
    h1: "Donner un espace à Hermes",
    tabs: ["Parler à Hermes", "Espaces"],
    current: "Espaces",
    h2: [
      "Ce qu’il faut",
      "Choisir l’espace",
      "Créer l’accès",
      "Installer le kit",
      "Configurer Hermes",
      "Vérifier la connexion",
      "Questions fréquentes",
      "Le kit Hermes",
    ],
    copied: "Copié",
    marked: "espace",
    faq: 7,
  },
];

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

const texts = (page: Page, selector: string): Promise<string[]> =>
  page
    .locator(selector)
    .evaluateAll((all) =>
      all.map((e) => (e.textContent ?? "").replace(/\s+/g, " ").trim()),
    );

for (const copy of [...LINK_PAGES, ...SPACE_PAGES]) {
  test.describe(copy.path, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(copy.path);
    });

    test("speaks the page language", async ({ page }) => {
      await expect(page.locator("html")).toHaveAttribute("lang", copy.lang);
      expect(await texts(page, "h1")).toEqual([copy.h1]);
      expect(await texts(page, "main h2")).toEqual(copy.h2);
    });

    test("ends no title with a full stop", async ({ page }) => {
      const titles = await texts(page, "h1, h2, h3");
      expect(titles.filter((t) => t.endsWith("."))).toEqual([]);
    });

    test("uses the site nav and footer", async ({ page }) => {
      const hermes = page
        .locator(`body > nav a[href='${copy.hermesNav}']`)
        .first();
      await expect(hermes).toHaveText("Hermes");
      await expect(hermes).toHaveAttribute("aria-current", "page");
      const active = await hermes.evaluate((e) => {
        const s = getComputedStyle(e);
        return { image: s.backgroundImage, color: s.color };
      });
      expect(active.image).toContain("linear-gradient");
      expect(active.color).toBe("rgb(255, 255, 255)");
      await expect(page.locator("footer")).toHaveCount(1);
      const other = page.locator("body > nav a[hreflang]");
      await expect(other).toHaveCount(1);
      expect(await other.getAttribute("href")).toMatch(
        new RegExp(`^${copy.other}/?$`),
      );
    });

    test("switches between the two Hermes pages", async ({ page }) => {
      expect(await texts(page, ".hermes-tabs a")).toEqual(copy.tabs);
      await expect(
        page.locator(".hermes-tabs a[aria-current='page']"),
      ).toHaveText(copy.current);
    });

    test("puts the cut-out mascot next to Hermes", async ({ page }) => {
      const mascot = page.locator(`header img[src='${MASCOT}']`);
      const agent = page.locator("header img[src='/images/hermes-agent.png']");
      await expect(mascot).toHaveCount(1);
      await expect(agent).toHaveCount(1);
      expect(
        await mascot.evaluate((e) => getComputedStyle(e).animationName),
      ).toBe("none");
      expect(await agent.evaluate((e) => getComputedStyle(e).boxShadow)).toBe(
        "none",
      );
      const edge = await agent.evaluate(async (image: HTMLImageElement) => {
        await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = image.naturalWidth;
        canvas.height = image.naturalHeight;
        const context = canvas.getContext("2d");
        context?.drawImage(image, 0, 0);
        const middle = Math.floor(canvas.width / 2);
        return Array.from(context?.getImageData(middle, 1, 1, 1).data ?? []);
      });
      expect(edge[0]).toBeGreaterThan(LIGHT);
    });

    test("offers the kit download", async ({ page }) => {
      const kit = page.locator(`a[href='${KIT}'][download]`);
      expect(await kit.count()).toBeGreaterThan(0);
    });

    test("uses the shared type, no serif", async ({ page }) => {
      const serif = await page.evaluate(() =>
        [...document.querySelectorAll("body *")].some((e) =>
          getComputedStyle(e).fontFamily.includes("Instrument Serif"),
        ),
      );
      expect(serif).toBe(false);
      await expect(page.locator("h1 em.say")).toHaveCount(0);
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

    test("does not scroll sideways", async ({ page }) => {
      await scrollThrough(page);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });
  });
}

test("keeps the GitHub star count live", async ({ page }) => {
  await page.route(GITHUB_REPO_API, (route) =>
    route.fulfill({ json: { stargazers_count: LIVE_STARS } }),
  );
  await page.goto("/hermes");
  await expect(page.locator("footer [data-stars]")).toHaveText(LIVE_LABEL);
});

for (const copy of LINK_PAGES) {
  test.describe(`${copy.path} steps`, () => {
    test("sets the hero word on the voice wave", async ({ page }) => {
      await page.goto(copy.path);
      const word = page.locator("h1 em.voice-word");
      await expect(word).toHaveText("FlowFlow");
      expect(await word.evaluate((e) => getComputedStyle(e).color)).toBe(
        "rgb(0, 0, 0)",
      );
      await expect(word.locator("canvas")).toHaveAttribute("data-wave", "bars");
    });

    test("walks through linking in order", async ({ page }) => {
      await page.goto(copy.path);
      expect(await texts(page, "main h3")).toEqual(copy.h3);
      await expect(page.locator("header a.btn-primary")).toHaveText(copy.cta);
      await expect(page.locator("header a.btn-primary")).toHaveAttribute(
        "href",
        "#relier",
      );
      expect(await texts(page, "#relier .hl")).toEqual(copy.proofs);
      await expect(page.locator(".needs")).toContainText(copy.manual);
      await expect(page.locator("#relier .num")).toHaveText(["01", "02", "03"]);
      await expect(
        page.locator(`main a[href='${copy.spaces}'].btn-primary`),
      ).toHaveCount(1);
      await expect(
        page.locator("main img[src='/logos/tailscale.svg']"),
      ).toHaveCount(1);
    });
  });
}

for (const copy of SPACE_PAGES) {
  test.describe(`${copy.path} guide`, () => {
    test("copies a prompt for Hermes", async ({ page, context }) => {
      await context.grantPermissions(["clipboard-read", "clipboard-write"]);
      await page.goto(copy.path);
      const button = page.locator("button[data-copy]").first();
      await button.click();
      await expect(button).toHaveText(copy.copied);
      const pasted = await page.evaluate(() => navigator.clipboard.readText());
      expect(pasted).toContain(KIT_PROMPT_FILE);
    });

    test("marks only the word space", async ({ page }) => {
      await page.goto(copy.path);
      await expect(page.locator("h1 .say")).toHaveText(copy.marked);
      await expect(page.locator("h1 .voice-word")).toHaveCount(0);
    });

    test("opens a common question", async ({ page }) => {
      await page.goto(copy.path);
      await expect(page.locator("#aide .qa")).toHaveCount(copy.faq);
      await page.locator("#aide .qa .q").first().click();
      await expect(page.locator("#aide .qa.open")).toHaveCount(1);
    });
  });
}
