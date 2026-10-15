// SPDX-License-Identifier: Apache-2.0
import { test, expect } from "@playwright/test";
import { mkdir } from "node:fs/promises";
const headers = { "X-Bizapp-Request": "1" };

async function seedApp(request, name, names = []) {
  const response = await request.post("/api/v1/apps", { headers, data: { name, fields: [{ key: "name", label: "名称", kind: "text", required: true }] } });
  expect(response.ok()).toBeTruthy();
  const app = await response.json();
  const records = [];
  for (const name of names) {
    const response = await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: { name } } });
    expect(response.ok()).toBeTruthy(); records.push(await response.json());
  }
  return { app, records };
}

async function noHorizontalOverflow(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBeTruthy();
}

test("create typed app, recover required input, search literal text, edit and delete", async ({ page }, info) => {
  await page.goto("/");
  await page.getByRole("button", { name: "＋ アプリを作る", exact: true }).click();
  const builder = page.getByRole("dialog", { name: "アプリを作る", exact: true });
  await builder.getByLabel("アプリ名", { exact: true }).fill(`UI flow ${info.project.name}`);
  await builder.getByLabel("項目名", { exact: true }).nth(1).fill("数量");
  await builder.getByLabel("種類", { exact: true }).nth(1).selectOption("number");
  await builder.getByLabel("必須", { exact: true }).nth(1).check();
  await builder.getByRole("button", { name: "＋ 項目を追加", exact: true }).click();
  await builder.getByLabel("項目名", { exact: true }).nth(2).fill("有効");
  await builder.getByLabel("種類", { exact: true }).nth(2).selectOption("boolean");
  await builder.getByRole("button", { name: "アプリを作成", exact: true }).click();
  await expect(page.getByRole("heading", { name: `UI flow ${info.project.name}`, exact: true })).toBeVisible();
  await page.getByRole("button", { name: "＋ レコードを追加", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "レコードを追加", exact: true });
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await expect(editor).toBeVisible();
  expect(await editor.locator("form").evaluate((form) => form.checkValidity())).toBeFalsy();
  const literal = "東京 <img src=x onerror=globalThis.__xss=1>";
  await editor.getByLabel(/^名称/).fill(literal);
  await editor.getByLabel(/^数量/).fill("9007199254740993");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await expect(editor.getByLabel(/^数量/)).toHaveValue("9007199254740993");
  expect(await editor.locator("form").evaluate((form) => form.checkValidity())).toBeFalsy();
  await editor.getByLabel(/^数量/).fill("0");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await expect(editor).not.toBeVisible();
  await expect(page.getByRole("cell", { name: literal, exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "0", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "いいえ", exact: true })).toBeVisible();
  await expect(page.locator("tbody img")).toHaveCount(0);
  expect(await page.evaluate(() => globalThis.__xss ?? null)).toBeNull();
  await page.getByLabel("レコードを検索", { exact: true }).fill("東京");
  await page.getByRole("button", { name: "検索", exact: true }).click();
  await expect(page.getByRole("cell", { name: literal, exact: true })).toBeVisible();
  await page.getByRole("button", { name: /^レコード.*を編集$/ }).click();
  const editing = page.getByRole("dialog", { name: "レコードを編集", exact: true });
  await editing.getByLabel(/^名称/).fill("東京 更新後");
  await editing.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("cell", { name: "東京 更新後", exact: true })).toBeVisible();
  await noHorizontalOverflow(page);
  await mkdir(".artifacts", { recursive: true });
  await page.screenshot({ path: `.artifacts/ui-${info.project.name}.png`, fullPage: true });
  await page.reload();
  await expect(page.getByRole("cell", { name: "東京 更新後", exact: true })).toBeVisible();
  await page.getByRole("button", { name: /^レコード.*を編集$/ }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("dialog").getByRole("button", { name: "削除", exact: true }).click();
  await expect(page.getByRole("cell", { name: "東京 更新後", exact: true })).toHaveCount(0);
  await expect(page.locator("#record-count")).toHaveText("このページに 0 件");
});

test("stale save preserves draft and reload recovers the latest revision", async ({ page, request }, info) => {
  const { app, records } = await seedApp(request, `Conflict ${info.project.name}`, ["Original"]);
  const record = records[0];
  await page.goto("/");
  await page.getByRole("button", { name: /^レコード.*を編集$/ }).click();
  const dialog = page.getByRole("dialog", { name: "レコードを編集", exact: true });
  await dialog.getByLabel(/^名称/).fill("My draft");
  const update = await request.put(`/api/v1/apps/${app.id}/records/${record.id}`, { headers, data: { revision: 1, values: { name: "Other editor" } } });
  expect(update.ok()).toBeTruthy();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog.getByRole("alert")).toContainText("ほかの変更が保存されています");
  await expect(dialog.getByLabel(/^名称/)).toHaveValue("My draft");
  await dialog.getByRole("button", { name: /^最新版を読み込む/ }).click();
  await expect(dialog.getByLabel(/^名称/)).toHaveValue("Other editor");
  await dialog.getByLabel(/^名称/).fill("Merged change");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("cell", { name: "Merged change", exact: true })).toBeVisible();
  const saved = await (await request.get(`/api/v1/apps/${app.id}/records/${record.id}`)).json();
  expect(saved.revision).toBe(3);
});

test("delayed page response cannot replace a newly selected app or corrupt its cursor", async ({ page, request }, info) => {
  const { app: first } = await seedApp(request, `Paging A ${info.project.name}`, Array.from({ length: 31 }, (_, i) => `A ${i}`));
  const { app: second } = await seedApp(request, `Paging B ${info.project.name}`, ["B only"]);
  await page.goto("/");
  await page.getByRole("button", { name: first.name, exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 30 件");
  let release, arrived;
  const gate = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrived = resolve; });
  await page.route(`**/api/v1/apps/${first.id}/records?*`, async (route) => {
    if (!route.request().url().includes("before=")) return route.continue();
    arrived(); await gate; await route.continue();
  });
  await page.getByRole("button", { name: "次のページ", exact: true }).click();
  await arrival;
  await page.getByRole("button", { name: second.name, exact: true }).click();
  await expect(page.getByRole("cell", { name: "B only", exact: true })).toBeVisible();
  const outdated = page.waitForResponse((response) => response.url().includes(`/apps/${first.id}/records?`) && response.url().includes("before="));
  release(); const response = await outdated; await response.finished();
  await page.evaluate(() => new Promise(requestAnimationFrame));
  await expect(page.getByRole("heading", { name: second.name, exact: true })).toBeVisible();
  await expect(page.locator("#page-number")).toHaveText("1 ページ");
  await expect(page.getByRole("cell", { name: "B only", exact: true })).toBeVisible();
});

test("keyboard dialogs return focus and narrow screens keep the page within its viewport", async ({ page, request }, info) => {
  await seedApp(request, `Keyboard ${info.project.name}`);
  await page.goto("/");
  await page.locator("#create-app").focus(); await page.keyboard.press("Enter");
  await expect(page.locator("#app-name")).toBeFocused();
  await noHorizontalOverflow(page);
  await page.keyboard.press("Escape");
  await expect(page.locator("#app-dialog")).not.toBeVisible();
  await expect(page.locator("#create-app")).toBeFocused();
  await page.locator("#new-record").focus(); await page.keyboard.press("Enter");
  await expect(page.locator("#record-fields input").first()).toBeFocused();
  await noHorizontalOverflow(page);
  await page.screenshot({ path: `.artifacts/dialog-${info.project.name}.png`, fullPage: true });
  await page.keyboard.press("Escape");
  await expect(page.locator("#new-record")).toBeFocused();
});


test("pending save locks cancellation and preserves the accepted draft", async ({ page, request }, info) => {
  const { app, records } = await seedApp(request, `Pending ${info.project.name}`, ["First", "Second"]);
  await page.goto("/");
  await page.getByRole("button", { name: `レコード${records[0].id}を編集`, exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "レコードを編集", exact: true });
  await dialog.getByLabel(/^名称/).fill("Accepted draft");
  let release, arrived;
  const gate = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrived = resolve; });
  await page.route(`**/api/v1/apps/${app.id}/records/${records[0].id}`, async (route) => {
    if (route.request().method() !== "PUT") return route.continue();
    arrived(); await gate; await route.continue();
  });
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await arrival;
  await expect(dialog.getByRole("button", { name: "キャンセル", exact: true })).toBeDisabled();
  await expect(dialog.getByLabel(/^名称/)).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  const completed = page.waitForResponse((response) => response.request().method() === "PUT" && response.url().endsWith(`/records/${records[0].id}`));
  release(); await completed;
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("cell", { name: "Accepted draft", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Second", exact: true })).toBeVisible();
});


test("date, select and multiline fields preserve typed values and lock during save", async ({ page, request }, info) => {
  await page.goto("/");
  await page.locator("#create-app").click();
  const builder = page.locator("#app-dialog");
  await builder.getByLabel("アプリ名", { exact: true }).fill(`Tasks ${info.project.name}`);
  for (const [label, kind] of [["期限", "date"], ["状況", "select"]]) {
    await builder.getByRole("button", { name: "＋ 項目を追加", exact: true }).click();
    await builder.getByLabel("項目名", { exact: true }).last().fill(label);
    await builder.getByLabel("種類", { exact: true }).last().selectOption(kind);
    await builder.getByLabel("必須", { exact: true }).last().check();
  }
  const options = builder.getByLabel("選択肢（1行に1つ）", { exact: true }).last();
  await options.fill("未対応\n未対応");
  await builder.getByRole("button", { name: "アプリを作成", exact: true }).click();
  await expect(builder.locator("#app-error")).toContainText("重複しないように");
  await expect(builder.getByLabel("アプリ名", { exact: true })).toHaveValue(`Tasks ${info.project.name}`);
  await options.fill("未対応\n完了\n<img src=x>");
  await noHorizontalOverflow(page);
  await page.screenshot({ path: `.artifacts/rich-builder-${info.project.name}.png`, fullPage: true });
  await builder.getByRole("button", { name: "アプリを作成", exact: true }).click();
  await expect(page.locator("#app-title")).toHaveText(`Tasks ${info.project.name}`);
  await page.locator("#new-record").click();
  const editor = page.locator("#record-dialog");
  await editor.getByLabel(/^名称/).fill("Quarterly report");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  expect(await editor.locator("form").evaluate((form) => form.checkValidity())).toBeFalsy();
  await editor.getByLabel(/^期限/).fill("2024-02-29");
  await editor.getByLabel(/^状況/).selectOption("未対応");
  const note = "一行目\n二行目 <script>literal</script>";
  await editor.getByLabel(/^メモ/).fill(note);
  await editor.getByLabel(/^メモ/).focus(); await page.keyboard.press("Tab");
  await expect(editor.getByLabel(/^期限/)).toBeFocused();
  await noHorizontalOverflow(page);
  await page.screenshot({ path: `.artifacts/rich-editor-${info.project.name}.png`, fullPage: true });
  let release, arrived;
  const gate = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrived = resolve; });
  await page.route("**/api/v1/apps/*/records", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    arrived(); await gate; await route.continue();
  });
  await editor.getByRole("button", { name: "保存", exact: true }).click(); await arrival;
  await expect(editor.getByLabel(/^状況/)).toBeDisabled();
  await expect(editor.getByLabel(/^メモ/)).toBeDisabled();
  await page.keyboard.press("Escape"); await expect(editor).toBeVisible();
  release(); await expect(editor).not.toBeVisible();
  await page.reload();
  await page.getByRole("button", { name: /^レコード.*を編集$/ }).click();
  await expect(editor.getByLabel(/^期限/)).toHaveValue("2024-02-29");
  await expect(editor.getByLabel(/^状況/)).toHaveValue("未対応");
  await expect(editor.getByLabel(/^メモ/)).toHaveValue(note);
  expect(await page.evaluate(() => document.querySelectorAll("#record-dialog script, #record-dialog img").length)).toBe(0);
  const apps = await (await request.get("/api/v1/apps")).json();
  const saved = await (await request.get(`/api/v1/apps/${apps[0].id}/records`)).json();
  expect(saved.records[0].values).toEqual({ field_1: "Quarterly report", field_2: note, field_3: "2024-02-29", field_4: "未対応" });
});


test("full Unicode choice lists and untouched API line endings survive form edits", async ({ page, request }, info) => {
  await page.goto("/"); await page.locator("#create-app").click();
  const builder = page.locator("#app-dialog");
  await builder.getByLabel("アプリ名", { exact: true }).fill(`Unicode ${info.project.name}`);
  await builder.getByRole("button", { name: "＋ 項目を追加", exact: true }).click();
  await builder.getByLabel("項目名", { exact: true }).last().fill("分類");
  await builder.getByLabel("種類", { exact: true }).last().selectOption("select");
  const choices = Array.from({ length: 50 }, (_, i) => String(i).padStart(2, "0") + "😀".repeat(78));
  const options = builder.getByLabel("選択肢（1行に1つ）", { exact: true }).last();
  await options.focus(); await page.keyboard.insertText(choices.join("\n"));
  await expect(options).toHaveValue(choices.join("\n"));
  await builder.getByRole("button", { name: "アプリを作成", exact: true }).click();
  await expect(page.locator("#app-title")).toHaveText(`Unicode ${info.project.name}`);
  const apps = await (await request.get("/api/v1/apps")).json(); const app = apps[0];
  expect(app.fields[2].options).toEqual(choices);
  const note = "line 1\r\n二行目\rthird";
  const response = await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: { field_1: "Original", field_2: note } } });
  expect(response.ok()).toBeTruthy(); const record = await response.json();
  await page.reload(); await page.getByRole("button", { name: `レコード${record.id}を編集`, exact: true }).click();
  const editor = page.locator("#record-dialog");
  await editor.getByLabel(/^メモ/).focus();
  await expect(editor.getByLabel(/^メモ/)).toHaveValue("line 1\n二行目\nthird");
  await editor.getByLabel(/^名称/).fill("Renamed");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await expect(editor).not.toBeVisible();
  const saved = await (await request.get(`/api/v1/apps/${app.id}/records/${record.id}`)).json();
  expect(saved.values.field_2).toBe(note); expect(saved.values.field_3).toBeNull();
});
