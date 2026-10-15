// SPDX-License-Identifier: Apache-2.0
import { test, expect } from "@playwright/test";
const headers = { "X-Bizapp-Request": "1" };
async function seed(request, name, values) {
  const response = await request.post("/api/v1/apps", { headers, data: { name, fields: [
    { key: "name", label: "名称", kind: "text", required: true },
    { key: "amount", label: "数量", kind: "number" },
    { key: "done", label: "完了", kind: "boolean" },
    { key: "due", label: "期限", kind: "date" },
    { key: "status", label: "状況", kind: "select", options: ["未対応", "完了", "<img src=x onerror=globalThis.__schemaXss=1>"] },
    { key: "note", label: "メモ", kind: "textarea" },
  ] } });
  expect(response.ok()).toBeTruthy(); const app = await response.json();
  for (const value of values) expect((await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: value } })).ok()).toBeTruthy();
  return app;
}
async function open(page) { await page.getByText("項目の条件で絞り込む", { exact: true }).click(); }
async function choose(page, field, value, select = false) {
  await page.getByLabel("絞り込む項目", { exact: true }).selectOption(field);
  const control = page.getByLabel("比較する値", { exact: true });
  if (select) await control.selectOption(value); else await control.fill(value);
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
}

test("typed filters apply to every page and CSV while drafts remain separate", async ({ page, request }, info) => {
  const app = await seed(request, `Filters ${info.project.name}`, [
    ...Array.from({ length: 37 }, (_, i) => ({ name: `東京 ${i}`, status: i < 32 ? "未対応" : "完了" })),
    { name: "大阪", status: "未対応" },
  ]);
  await page.goto("/"); await open(page);
  await page.getByLabel("レコードを検索", { exact: true }).fill("東京");
  await choose(page, "status", "未対応", true);
  await expect(page.locator("#query-summary")).toContainText("状況（一致：未対応）");
  await expect(page.locator("#record-count")).toHaveText("このページに 30 件");
  await page.getByLabel("比較する値", { exact: true }).selectOption("完了");
  await page.getByRole("button", { name: "次のページ", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 2 件");
  await expect(page.locator("#query-summary")).toContainText("状況（一致：未対応）");
  const downloading = page.waitForEvent("download"); await page.getByRole("button", { name: "CSVを出力", exact: true }).click();
  const chunks = []; for await (const chunk of await (await downloading).createReadStream()) chunks.push(chunk);
  const csv = Buffer.concat(chunks).toString("utf8");
  expect(csv.split("\r\n").filter(Boolean)).toHaveLength(33); expect(csv).not.toContain("大阪"); expect(csv).not.toContain('"完了"\r\n');
  expect(csv).toContain("東京 31"); expect(csv).toContain("東京 0");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBeTruthy();
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: `.artifacts/filters-${info.project.name}.png`, fullPage: true });
  await page.getByRole("button", { name: "条件を解除", exact: true }).click();
  await expect(page.locator("#query-summary")).toHaveText("適用中：文字検索「東京」");
  await expect(page.locator("#page-number")).toHaveText("1 ページ");
  await page.getByRole("button", { name: "クリア", exact: true }).click();
  await expect(page.locator("#query-summary")).toHaveText("適用中：すべてのレコード");
});

test("number zero, false, date, multiline text, and empty fields use typed controls", async ({ page, request }, info) => {
  await seed(request, `Types ${info.project.name}`, [
    { name: "Zero", amount: 0, done: false, due: "2024-02-29", note: "メモ" },
    { name: "One", amount: 1, done: true, note: "" },
    { name: "Missing" },
  ]);
  await page.goto("/"); await open(page);
  for (const [field, value, select, expected] of [["amount", "0", false, "Zero"], ["done", "false", true, "Zero"], ["due", "2024-02-29", false, "Zero"], ["note", "メモ", false, "Zero"], ["name", "One", false, "One"]]) {
    await choose(page, field, value, select);
    await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
    await expect(page.getByRole("cell", { name: expected, exact: true })).toBeVisible();
  }
  await page.getByLabel("絞り込む項目", { exact: true }).selectOption("due");
  await page.getByLabel("条件", { exact: true }).selectOption("empty");
  await expect(page.getByLabel("比較する値", { exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 2 件");
  await expect(page.getByRole("cell", { name: "Missing", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "One", exact: true })).toBeVisible();
});

test("filter errors retain controls and allow retry without exporting the wrong view", async ({ page, request }, info) => {
  const app = await seed(request, `Retry filters ${info.project.name}`, [{ name: "Zero", amount: 0 }]);
  await page.goto("/"); await open(page);
  await page.getByLabel("絞り込む項目", { exact: true }).selectOption("amount");
  await page.getByLabel("比較する値", { exact: true }).fill("9007199254740993");
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  expect(await page.locator("#search-form").evaluate((form) => form.checkValidity())).toBeFalsy();
  const pattern = `**/api/v1/apps/${app.id}/records?*`;
  await page.route(pattern, (route) => route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: { code: "unavailable" } }) }));
  await page.getByLabel("比較する値", { exact: true }).fill("0");
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.locator("#status")).toContainText("少し待って再試行");
  await expect(page.locator("#query-summary")).toContainText("一覧の取得に失敗");
  await expect(page.getByLabel("比較する値", { exact: true })).toHaveValue("0");
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeDisabled();
  await page.unroute(pattern);
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeEnabled();
  await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
});

test("app switches clear filter drafts and delayed filtered results cannot replace the new app", async ({ page, request }, info) => {
  const first = await seed(request, `Filters A ${info.project.name}`, [{ name: "A", done: false }]);
  const second = await seed(request, `Filters B ${info.project.name}`, [{ name: "B", done: true }]);
  await page.goto("/"); await page.getByRole("button", { name: first.name, exact: true }).click(); await open(page);
  let release, arrive;
  const gate = new Promise((resolve) => { release = resolve; }); const arrival = new Promise((resolve) => { arrive = resolve; });
  await page.route(`**/api/v1/apps/${first.id}/records?*`, async (route) => { if (!route.request().url().includes("filter=")) return route.continue(); arrive(); await gate; await route.continue(); });
  await choose(page, "done", "false", true); await arrival;
  await page.getByRole("button", { name: second.name, exact: true }).click();
  await expect(page.getByRole("cell", { name: "B", exact: true })).toBeVisible();
  const finished = page.waitForResponse((response) => response.url().includes(`/apps/${first.id}/records?`) && response.url().includes("filter="));
  release(); await (await finished).finished();
  await expect(page.locator("#query-summary")).toHaveText("適用中：すべてのレコード");
  await expect(page.getByLabel("絞り込む項目", { exact: true })).toHaveValue("");
  await expect(page.getByRole("cell", { name: "B", exact: true })).toBeVisible();
});

test("schema labels and choice filters stay literal through editor and settings", async ({ page, request }, info) => {
  const markup = "<img src=x onerror=globalThis.__schemaXss=1>";
  const app = await seed(request, `${markup} ${info.project.name}`, [{ name: "Literal", status: markup }]);
  expect((await request.put(`/api/v1/apps/${app.id}`, { headers, data: { revision: app.revision, name: app.name, fields: app.fields.map((field) => field.key === "status" ? { ...field, label: markup } : field) } })).ok()).toBeTruthy();
  await page.goto("/"); await open(page); await choose(page, "status", markup, true);
  await expect(page.getByRole("heading", { name: app.name, exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: markup, exact: true })).toBeVisible();
  await page.getByRole("button", { name: /^レコード.*を編集$/ }).click();
  await expect(page.getByLabel(markup, { exact: true })).toHaveValue(markup);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "フォーム設定", exact: true }).click();
  await expect(page.getByLabel("アプリ名", { exact: true })).toHaveValue(app.name);
  expect(await page.locator("#business-apps-root img, #business-apps-root script").count()).toBe(0);
  expect(await page.evaluate(() => globalThis.__schemaXss ?? null)).toBeNull();
});

test("multiline equality preserves newlines in the filter draft and submitted result", async ({ page, request }, info) => {
  await seed(request, `Multiline filter ${info.project.name}`, [{ name: "Newlines", note: "a\nb" }, { name: "Single line", note: "ab" }]);
  await page.goto("/"); await open(page);
  await choose(page, "note", "a\nb");
  await expect(page.getByLabel("比較する値", { exact: true })).toHaveValue("a\nb");
  await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
  await expect(page.getByRole("cell", { name: "Newlines", exact: true })).toBeVisible();
});

test("exact empty text differs from the broader empty-cell condition", async ({ page, request }, info) => {
  await seed(request, `Empty equality ${info.project.name}`, [{ name: "Explicit empty", note: "" }, { name: "Null", note: null }, { name: "Missing" }]);
  await page.goto("/"); await open(page);
  await choose(page, "note", "");
  await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
  await expect(page.getByRole("cell", { name: "Explicit empty", exact: true })).toBeVisible();
  await page.getByLabel("条件", { exact: true }).selectOption("empty");
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 3 件");
});

test("failed conditions cannot reuse the preceding result's next-page cursor", async ({ page, request }, info) => {
  const app = await seed(request, `Cursor retry ${info.project.name}`, Array.from({ length: 35 }, (_, i) => ({ name: `Row ${i}`, done: false })));
  await page.goto("/");
  await expect(page.getByRole("button", { name: "次のページ", exact: true })).toBeEnabled();
  await open(page);
  const pattern = `**/api/v1/apps/${app.id}/records?*`;
  await page.route(pattern, (route) => route.fulfill({ status: 503, contentType: "application/json", body: '{"error":{"code":"unavailable"}}' }));
  await choose(page, "done", "true", true);
  await expect(page.locator("#query-summary")).toContainText("一覧の取得に失敗");
  await expect(page.getByRole("button", { name: "次のページ", exact: true })).toBeDisabled();
  await expect(page.getByRole("button", { name: "前のページ", exact: true })).toBeDisabled();
  await page.unroute(pattern);
  await choose(page, "done", "false", true);
  await expect(page.getByRole("cell", { name: "Row 34", exact: true })).toBeVisible();
});
