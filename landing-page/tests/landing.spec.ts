import { expect, type Page, test } from "@playwright/test";

const PAGES = [
  {
    path: "/",
    h1: "A word away",
    sub: "Speak, FlowFlow runs it",
    steps: ["Speak", "Transcribe", "Organize", "Recall"],
    agentic: "From word",
    teams: "You, your team, your agent.",
    trust: "What stays,",
    free: "Free for now",
    calendar: "Google Calendar",
  },
  {
    path: "/fr/",
    h1: "À portée de voix",
    sub: "Parle, FlowFlow exécute",
    steps: ["Parle", "Transcrit", "Range", "Retrouve"],
    agentic: "Du dire",
    teams: "Toi, ton équipe, ton agent.",
    trust: "Ce qui reste,",
    free: "Gratuite pour l'instant",
    calendar: "Google Agenda",
  },
];

const FORBIDDEN = ["REAL GIF", "REAL SCREENSHOT", "GIF RÉEL", "100% Rust"];
const SOON = /\b(soon|bientôt)\b/i;
const WHITE = "rgb(255, 255, 255)";
const BLACK = "rgb(0, 0, 0)";
const MIN_CLIPS = 7;
const CONNECTORS = 6;

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
      await expect(page.locator("#product h3")).toHaveText(copy.steps);
      await expect(page.locator("#agentic h2")).toContainText(copy.agentic);
      await expect(page.locator("#teams h2")).toContainText(copy.teams);
      await expect(page.locator("#trust h2")).toContainText(copy.trust);
      await expect(page.locator("#trust dl")).toContainText(copy.free);
    });

    test("has no empty frame, form or soon badge", async ({ page }) => {
      const text = (await page.locator("body").innerText()) ?? "";
      for (const word of FORBIDDEN) expect(text).not.toContain(word);
      expect(text).not.toMatch(SOON);
      await expect(page.locator("form")).toHaveCount(0);
      await expect(page.locator(".marquee")).toHaveCount(0);
    });

    test("shows the real connectors", async ({ page }) => {
      const chips = page.locator("#agentic li");
      await expect(chips).toHaveCount(CONNECTORS);
      await expect(chips.nth(2)).toContainText(copy.calendar);
      await expect(page.locator("#agentic li svg")).toHaveCount(CONNECTORS);
    });

    test("uses white on orange and black on white", async ({ page }) => {
      expect(await colorOf(page, "#agentic h2")).toBe(WHITE);
      expect(await colorOf(page, "#agentic h2 em")).toBe(WHITE);
      expect(await colorOf(page, "#agentic .sub")).toBe(WHITE);
      expect(await colorOf(page, ".btn-primary")).toBe(WHITE);
      expect(await colorOf(page, '.offer[data-tone="orange"] h3')).toBe(WHITE);
      expect(await colorOf(page, "#trust .leave h3")).toBe(WHITE);
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

    test("does not scroll sideways", async ({ page }) => {
      await scrollThrough(page);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });
  });
}
