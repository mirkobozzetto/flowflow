import { expect, type Page, test } from "@playwright/test";

const PAGES = [
  {
    path: "/",
    h1: "A word away",
    sub: "Speak, FlowFlow runs it",
    day: "Less admin,",
    choice: "As a team",
    caption: "Your meeting, already summed up",
    teams: "You, your team, your agent.",
    trust: "What stays,",
    free: "Free for now",
  },
  {
    path: "/fr/",
    h1: "À portée de voix",
    sub: "Parle, FlowFlow exécute",
    day: "Moins d'administratif,",
    choice: "En équipe",
    caption: "Ta réunion, déjà résumée",
    teams: "Toi, ton équipe, ton agent.",
    trust: "Ce qui reste,",
    free: "Gratuite pour l'instant",
  },
];

const FORBIDDEN = ["REAL GIF", "REAL SCREENSHOT", "SCREENSHOT", "100% Rust"];
const SOON = /\b(soon|bientôt)\b/i;

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

for (const copy of PAGES) {
  test.describe(copy.path, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(copy.path);
    });

    test("keeps the headline and shows the exact copy", async ({ page }) => {
      await expect(page.locator("h1")).toHaveText(copy.h1);
      await expect(page.locator("header p").first()).toContainText(copy.sub);
      await expect(page.locator("#product h2")).toContainText(copy.day);
      await expect(page.locator("#teams h2")).toContainText(copy.teams);
      await expect(page.locator("#trust h2")).toContainText(copy.trust);
      await expect(page.locator("#trust dl")).toContainText(copy.free);
    });

    test("has no empty frame, form or soon badge", async ({ page }) => {
      const text = (await page.locator("body").innerText()) ?? "";
      for (const word of FORBIDDEN) expect(text).not.toContain(word);
      expect(text).not.toMatch(SOON);
      await expect(page.locator("form")).toHaveCount(0);
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

    test("plays the promo, muted, and switches clips", async ({ page }) => {
      const video = page.locator("[data-stage-video]");
      await expect
        .poll(() => video.evaluate((v: HTMLVideoElement) => v.currentTime))
        .toBeGreaterThan(0.2);
      expect(await video.evaluate((v: HTMLVideoElement) => v.muted)).toBe(true);

      await page.getByRole("button", { name: copy.choice }).click();
      await expect(page.locator("[data-caption]:visible")).toContainText(
        copy.caption,
      );
      await expect(video).toHaveAttribute("src", /meeting\.mp4$/);
      await expect
        .poll(() => video.evaluate((v: HTMLVideoElement) => v.currentTime))
        .toBeGreaterThan(0.2);
    });

    test("turns the sound on", async ({ page }) => {
      await page.locator("[data-sound]").click();
      const video = page.locator("[data-stage-video]");
      expect(await video.evaluate((v: HTMLVideoElement) => v.muted)).toBe(
        false,
      );
    });

    test("every lazy clip plays once in view", async ({ page }) => {
      const clips = page.locator("video[data-lazy]");
      const count = await clips.count();
      expect(count).toBeGreaterThanOrEqual(5);
      for (let i = 0; i < count; i += 1) {
        const clip = clips.nth(i);
        await clip.evaluate((v) => v.scrollIntoView({ block: "center" }));
        await expect
          .poll(() => clip.evaluate((v: HTMLVideoElement) => v.currentTime), {
            timeout: 8000,
          })
          .toBeGreaterThan(0.1);
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

test("falls back to a play button when autoplay is refused", async ({
  page,
}) => {
  await page.addInitScript(() => {
    HTMLMediaElement.prototype.play = () =>
      Promise.reject(new Error("autoplay refused"));
  });
  await page.goto("/fr/");
  await expect(page.locator("[data-play]")).toBeVisible();
});
