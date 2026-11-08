import { defineConfig } from "@playwright/test";

// Another worktree can run its suite at the same time on its own port.
const PORT = Number(process.env.TEST_PORT ?? 4399);

// The bundled Chromium has no H.264: videos only play in real Chrome.
export default defineConfig({
  testDir: "tests",
  reporter: "list",
  fullyParallel: true,
  use: { baseURL: `http://localhost:${PORT}`, channel: "chrome" },
  projects: [
    { name: "mac", use: { viewport: { width: 1440, height: 900 } } },
    {
      name: "iphone",
      use: {
        viewport: { width: 390, height: 844 },
        isMobile: true,
        hasTouch: true,
      },
    },
  ],
  webServer: {
    command: `bun run build && bun run preview --port ${PORT}`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
