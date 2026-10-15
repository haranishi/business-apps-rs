// SPDX-License-Identifier: Apache-2.0
import { test, expect } from "@playwright/test";

test("Rust UI actually loads WebAssembly under the scoped content policy", async ({ page }) => {
  const errors=[]; page.on("pageerror", (error)=>errors.push(error.message));
  const module=page.waitForResponse((response)=>response.url().endsWith("/assets/business_apps_ui_bg.wasm"));
  const document=await page.goto("/");
  const wasm=await module;
  expect(wasm.ok()).toBeTruthy(); expect(wasm.headers()["content-type"]).toBe("application/wasm");
  expect((await wasm.body()).subarray(0,4).toString()).toBe("\u0000asm");
  const policy=document.headers()["content-security-policy"];
  expect(policy).toContain("'wasm-unsafe-eval'"); expect(policy).not.toContain("'unsafe-eval'"); expect(policy).not.toContain("'unsafe-inline'");
  await expect(page.locator("body")).toHaveAttribute("data-ui-runtime","rust-wasm");
  await expect(page.locator("#create-app")).toBeVisible(); await expect(page.locator("#boot-status")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("a failed WASM download explains recovery and reload restores the UI", async ({ page }) => {
  await page.route("**/assets/business_apps_ui_bg.wasm",(route)=>route.abort("failed"));
  await page.goto("/"); await expect(page.locator("#boot-status")).toContainText("画面を起動できませんでした");
  await expect(page.locator("#create-app")).toHaveCount(0);
  await page.unroute("**/assets/business_apps_ui_bg.wasm"); await page.reload();
  await expect(page.locator("body")).toHaveAttribute("data-ui-runtime","rust-wasm");
  await expect(page.locator("#create-app")).toBeVisible(); await expect(page.locator("#boot-status")).toHaveCount(0);
});


test("a failed glue download shows recovery and reload restores the UI", async ({ page }) => {
  await page.route("**/assets/business_apps_ui.js", (route) => route.abort("failed"));
  await page.goto("/");
  await expect(page.locator("#boot-status")).toContainText("画面を起動できませんでした");
  await expect(page.locator("#create-app")).toHaveCount(0);
  await page.unroute("**/assets/business_apps_ui.js");
  await page.reload();
  await expect(page.locator("body")).toHaveAttribute("data-ui-runtime", "rust-wasm");
  await expect(page.locator("#create-app")).toBeVisible();
  await expect(page.locator("#boot-status")).toHaveCount(0);
});

test("an installation failure removes unusable controls and retains recovery text", async ({ page }) => {
  await page.addInitScript(() => { window.ResizeObserver = undefined; });
  await page.goto("/");
  await expect(page.locator("#boot-status")).toContainText("画面を起動できませんでした");
  await expect(page.locator("#business-apps-root")).toHaveCount(0);
  await expect(page.locator("#create-app")).toHaveCount(0);
  await expect(page.locator("body")).not.toHaveAttribute("data-ui-runtime", "rust-wasm");
});


test("initial app loading shows progress before the empty or populated view", async ({ page }) => {
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  await page.route("**/api/v1/apps", async (route) => { await gate; await route.continue(); });
  await page.goto("/");
  await expect(page.locator("body")).toHaveAttribute("data-ui-runtime", "rust-wasm");
  await expect(page.locator("#status")).toHaveText("アプリを読み込み中…");
  await expect(page.locator("#welcome")).toBeHidden();
  await expect(page.locator("#create-app")).toBeDisabled();
  release();
  await expect(page.locator("#create-app")).toBeEnabled();
  await expect(page.locator("#status")).not.toHaveText("アプリを読み込み中…");
});

test("an initial app-list error explains recovery without displaying an empty workspace", async ({ page }) => {
  await page.route("**/api/v1/apps", (route) => route.fulfill({ status: 503, json: { error: { message: "Synthetic unavailable" } } }));
  await page.goto("/");
  await expect(page.locator("#status")).toContainText("アプリを読み込めませんでした");
  await expect(page.locator("#welcome")).toBeHidden();
  await expect(page.locator("#create-app")).toBeEnabled();
  await page.unroute("**/api/v1/apps");
  await page.reload();
  await expect(page.locator("#create-app")).toBeEnabled();
  await expect(page.locator("#status")).not.toContainText("アプリを読み込めませんでした");
});
