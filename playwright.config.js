// SPDX-License-Identifier: Apache-2.0
import { defineConfig } from "@playwright/test";
const port = process.env.BIZAPP_TEST_PORT || "8091";
export default defineConfig({
  testDir: "./tests", fullyParallel: false, workers: 1, retries: 0,
  forbidOnly: !!process.env.CI, timeout: 30000,
  outputDir: ".artifacts/test-results",
  reporter: [["list"], ["html", { outputFolder: ".artifacts/playwright-report", open: "never" }]],
  use: { baseURL: `http://127.0.0.1:${port}`, trace: "retain-on-failure", screenshot: "only-on-failure" },
  projects: [
    { name: "mobile", use: { viewport: { width: 390, height: 844 } } },
    { name: "tablet", use: { viewport: { width: 768, height: 1024 } } },
    { name: "desktop", use: { viewport: { width: 1440, height: 1000 } } },
  ],
  webServer: { command: "node scripts/e2e_server.mjs", url: `http://127.0.0.1:${port}/health`, timeout: 120000, reuseExistingServer: false },
});
