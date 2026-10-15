// SPDX-License-Identifier: Apache-2.0
import { test, expect } from "@playwright/test";

const headers = { "X-Bizapp-Request": "1" };
async function seed(request, name, count = 1) {
  const response = await request.post("/api/v1/apps", { headers, data: { name, fields: [{ key: "name", label: "名称", kind: "text" }] } });
  expect(response.ok()).toBeTruthy();
  const app = await response.json();
  for (let index = 0; index < count; index++) {
    expect((await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: { name: `東京 ${index}` } } })).ok()).toBeTruthy();
  }
  return app;
}
async function contents(download) {
  expect(await download.failure()).toBeNull();
  const chunks = [];
  for await (const chunk of await download.createReadStream()) chunks.push(chunk);
  return Buffer.concat(chunks);
}

test("CSV exports every matching page using the applied search and preserves the workspace", async ({ page, request }, info) => {
  const app = await seed(request, `CSV ${info.project.name}`, 35);
  expect((await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: { name: "大阪" } } })).ok()).toBeTruthy();
  await page.goto("/");
  await page.getByLabel("レコードを検索", { exact: true }).fill("東京");
  await page.getByRole("button", { name: "検索", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 30 件");
  await page.getByLabel("レコードを検索", { exact: true }).fill("大阪");
  await page.getByRole("button", { name: "次のページ", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 5 件");
  const downloading = page.waitForEvent("download");
  await page.getByRole("button", { name: "CSVを出力", exact: true }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe(`app-${app.id}-records.csv`);
  const bytes = await contents(download);
  expect([...bytes.subarray(0, 3)]).toEqual([0xef, 0xbb, 0xbf]);
  const csv = bytes.toString("utf8");
  expect(csv.split("\r\n").filter(Boolean)).toHaveLength(36);
  expect(csv).toContain('"名称"');
  expect(csv).toContain('"東京 34"'); expect(csv).toContain('"東京 0"');
  expect(csv).not.toContain("大阪");
  await expect(page.locator("#export-status")).toContainText("CSVを出力しました");
  await expect(page.locator("#page-number")).toHaveText("2 ページ");
  await expect(page.getByLabel("レコードを検索", { exact: true })).toHaveValue("大阪");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBeTruthy();
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: `.artifacts/csv-${info.project.name}.png`, fullPage: true });
  await page.getByRole("button", { name: "クリア", exact: true }).click();
  await expect(page.locator("#page-number")).toHaveText("1 ページ");
  await expect(page.getByRole("cell", { name: "大阪", exact: true })).toBeVisible();
});

test("CSV errors retain the search and support an explicit retry", async ({ page, request }, info) => {
  const app = await seed(request, `CSV retry ${info.project.name}`);
  await page.goto("/");
  const pattern = `**/api/v1/apps/${app.id}/records.csv?*`;
  await page.route(pattern, (route) => route.fulfill({ status: 413, contentType: "application/json", body: JSON.stringify({ error: { code: "export_limit", message: "limit" } }) }));
  await page.getByRole("button", { name: "CSVを出力", exact: true }).click();
  await expect(page.locator("#export-status")).toContainText("検索で絞り込んでから再試行");
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeEnabled();
  await expect(page.getByRole("cell", { name: "東京 0", exact: true })).toBeVisible();
  await page.unroute(pattern);
  const downloading = page.waitForEvent("download");
  await page.getByRole("button", { name: "CSVを出力", exact: true }).click();
  expect((await contents(await downloading)).toString("utf8")).toContain("東京 0");
});

test("pending CSV locks duplicate requests and discards a response after app selection changes", async ({ page, request }, info) => {
  const first = await seed(request, `CSV delayed A ${info.project.name}`);
  const second = await seed(request, `CSV delayed B ${info.project.name}`);
  await page.goto("/");
  await page.getByRole("button", { name: first.name, exact: true }).click();
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeEnabled();
  let release, arrive;
  const gate = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrive = resolve; });
  await page.route(`**/api/v1/apps/${first.id}/records.csv?*`, async (route) => { arrive(); await gate; await route.continue(); });
  await page.getByRole("button", { name: "CSVを出力", exact: true }).click(); await arrival;
  await expect(page.locator("#export-status")).toHaveText("CSVを準備中…");
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: second.name, exact: true }).click();
  await expect(page.locator("#app-title")).toHaveText(second.name);
  const unexpectedDownload = page.waitForEvent("download", { timeout: 750 }).then(() => true, () => false);
  release();
  await expect(page.getByRole("button", { name: "CSVを出力", exact: true })).toBeEnabled();
  expect(await unexpectedDownload).toBeFalsy();
  await expect(page.locator("#export-status")).toHaveText("");
});

test("CSV completion preserves a record draft and returns focus to its invoking edit button", async ({ page, request }, info) => {
  const app = await seed(request, `CSV focus ${info.project.name}`);
  await page.goto("/");
  let release, arrive;
  const gate = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrive = resolve; });
  await page.route(`**/api/v1/apps/${app.id}/records.csv?*`, async (route) => { arrive(); await gate; await route.continue(); });
  const downloading = page.waitForEvent("download");
  await page.getByRole("button", { name: "CSVを出力", exact: true }).click(); await arrival;
  const invoking = page.getByRole("button", { name: /^レコード.*を編集$/ });
  await invoking.click();
  await page.getByLabel("名称", { exact: true }).fill("Unsaved during export");
  release(); await contents(await downloading);
  await expect(page.getByLabel("名称", { exact: true })).toHaveValue("Unsaved during export");
  await page.keyboard.press("Escape");
  await expect(invoking).toBeFocused();
});
