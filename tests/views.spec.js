// SPDX-License-Identifier: Apache-2.0
import { test, expect } from "@playwright/test";
const headers = { "X-Bizapp-Request": "1" };
const picker = (page) => page.getByLabel("表示する一覧", { exact: true });
const dialog = (page) => page.locator("#view-dialog");
async function seed(request, name, values) {
  const response = await request.post("/api/v1/apps", { headers, data: { name, fields: [
    { key: "name", label: "名称", kind: "text", required: true },
    { key: "amount", label: "数量", kind: "number" },
    { key: "done", label: "完了", kind: "boolean" },
    { key: "due", label: "期限", kind: "date" },
    { key: "status", label: "状況", kind: "select", options: ["未対応", "完了", "\uFEFFOpen"] },
    { key: "note", label: "メモ", kind: "textarea" },
  ] } });
  expect(response.ok()).toBeTruthy(); const app = await response.json();
  const records = [];
  for (const value of values) {
    const response = await request.post(`/api/v1/apps/${app.id}/records`, { headers, data: { values: value } });
    expect(response.ok()).toBeTruthy(); records.push(await response.json());
  }
  return { app, records };
}
async function createView(request, app, definition) {
  const response = await request.post(`/api/v1/apps/${app.id}/views`, { headers, data: definition });
  expect(response.status()).toBe(201); return response.json();
}
async function catalog(request, app) {
  const response = await request.get(`/api/v1/apps/${app.id}/views`);
  expect(response.ok()).toBeTruthy(); return response.json();
}
async function choose(page, view) {
  await expect(picker(page)).toBeEnabled();
  const fetched = page.waitForResponse((response) => response.url().includes("/records?") && response.request().method() === "GET");
  await picker(page).selectOption(String(view?.id ?? "")); await (await fetched).finished();
  await expect(page.locator("#query-summary")).toContainText("適用中：");
  await expect(page.locator("#export-csv")).toBeEnabled();
}
function gate() {
  let release, arrive;
  const wait = new Promise((resolve) => { release = resolve; });
  const arrival = new Promise((resolve) => { arrive = resolve; });
  return { wait, arrival, release, arrive };
}
async function pendingLocked(page) {
  for (const control of await dialog(page).locator("input, button").all()) await expect(control).toBeDisabled();
  await page.keyboard.press("Escape"); await expect(dialog(page)).toBeVisible();
}

test("save applied conditions and ordered columns, reopen, edit hidden fields, and export every field/page", async ({ page, request }, info) => {
  const { app } = await seed(request, `Saved views ${info.project.name}`, [
    ...Array.from({ length: 35 }, (_, index) => ({ name: `東京 ${index}`, done: false, amount: 0, status: "未対応", note: "保存するメモ" })),
    { name: "大阪", done: true },
  ]);
  const errors = []; page.on("pageerror", (error) => errors.push(error.message));
  // Keep screenshots focused on this synthetic app; other tests share the server.
  // The Rust WASM UI still sends view and record requests to the real server.
  await page.route("**/api/v1/apps", (route) => route.fulfill({ json: [app] }));
  await page.goto("/"); await expect(picker(page)).toBeEnabled();
  await page.getByText("項目の条件で絞り込む", { exact: true }).click();
  await page.getByLabel("レコードを検索", { exact: true }).fill("東京");
  await page.getByLabel("絞り込む項目", { exact: true }).selectOption("done");
  await page.getByLabel("比較する値", { exact: true }).selectOption("false");
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.locator("#query-summary")).toHaveText("適用中：文字検索「東京」 ／ 完了（一致：いいえ）");
  await page.getByLabel("レコードを検索", { exact: true }).fill("未適用の入力");
  await page.getByRole("button", { name: "この条件で一覧を保存", exact: true }).click();
  await expect(page.getByLabel("一覧名", { exact: true })).toBeFocused();
  await page.getByLabel("一覧名", { exact: true }).fill("東京の未完了");
  await expect(page.locator("#view-query")).not.toContainText("未適用");
  for (const label of ["数量", "期限", "状況", "メモ"]) await dialog(page).getByRole("checkbox", { name: label, exact: true }).uncheck();
  await dialog(page).getByRole("button", { name: "完了を上へ", exact: true }).click();
  await dialog(page).getByRole("button", { name: "完了を上へ", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(dialog(page).getByRole("checkbox", { name: "完了", exact: true })).toBeFocused();
  await expect(dialog(page).getByRole("button", { name: "完了を上へ", exact: true })).toBeDisabled();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBeTruthy();
  await page.evaluate(() => scrollTo(0, 0));
  await dialog(page).evaluate((node) => { node.scrollTop = 0; });
  await page.screenshot({ path: `.artifacts/views-dialog-${info.project.name}.png` });
  await page.locator("#confirm-view").scrollIntoViewIfNeeded();
  await expect(page.locator("#confirm-view")).toBeInViewport();
  await expect(dialog(page).getByRole("button", { name: "キャンセル", exact: true })).toBeInViewport();
  await page.screenshot({ path: `.artifacts/views-dialog-actions-${info.project.name}.png` });
  await dialog(page).getByRole("button", { name: "一覧を保存", exact: true }).click();
  await expect(dialog(page)).not.toBeVisible();
  await expect(page.locator("#table-head th")).toHaveText(["完了", "名称", "操作"]);
  const [view] = await catalog(request, app);
  expect(view.definition).toEqual({ name: "東京の未完了", query: "東京", filter: { field: "done", value: false }, columns: ["done", "name"] });
  await page.getByRole("button", { name: "次のページ", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 5 件");
  const downloading = page.waitForEvent("download"); await page.locator("#export-csv").click();
  const chunks = []; for await (const chunk of await (await downloading).createReadStream()) chunks.push(chunk);
  const csv = Buffer.concat(chunks).toString("utf8");
  expect(csv.split("\r\n").filter(Boolean)).toHaveLength(36);
  expect(csv).toContain('"名称","数量","完了","期限","状況","メモ"');
  expect(csv).toContain("保存するメモ"); expect(csv).toContain("東京 0"); expect(csv).toContain("東京 34"); expect(csv).not.toContain("大阪");
  const editing = page.getByRole("button", { name: /^レコード.*を編集$/ }).first();
  const recordId = await editing.getAttribute("data-record-id"); await editing.click();
  await expect(page.locator("#record-note")).toHaveValue("保存するメモ");
  await page.locator("#record-name").fill("東京 更新後"); await page.locator("#save-record").click();
  await expect(page.locator("#record-dialog")).not.toBeVisible();
  expect((await (await request.get(`/api/v1/apps/${app.id}/records/${recordId}`)).json()).values.note).toBe("保存するメモ");
  await page.reload(); await expect(picker(page)).toBeEnabled();
  await expect(picker(page)).toHaveValue(""); await expect(page.locator("#table-head th")).toHaveCount(7);
  await choose(page, view); await expect(page.locator("#table-head th")).toHaveText(["完了", "名称", "操作"]);
  await expect(page.getByLabel("レコードを検索", { exact: true })).toHaveValue("東京");
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: `.artifacts/views-${info.project.name}.png` });
  await page.screenshot({ path: `.artifacts/views-full-${info.project.name}.png`, fullPage: true });
  await choose(page, null); await expect(page.locator("#query-summary")).toHaveText("適用中：すべてのレコード");
  await expect(page.locator("#table-head th")).toHaveCount(7); await expect(page.locator("#page-number")).toHaveText("1 ページ");
  expect(errors).toEqual([]);
});

test("update/delete conflicts preserve drafts, explicit reload replaces them, and deletion preserves records", async ({ page, request }, info) => {
  const { app } = await seed(request, `View revisions ${info.project.name}`, [{ name: "Pending", done: false }, { name: "Finished", done: true }]);
  const view = await createView(request, app, { name: "未完了", columns: ["name"], filter: { field: "done", value: false } });
  await page.goto("/"); await choose(page, view);
  await page.getByLabel("比較する値", { exact: true }).selectOption("true");
  await page.getByRole("button", { name: "条件を適用", exact: true }).click();
  await expect(page.locator("#view-modified")).toContainText("条件を変更しています");
  await page.locator("#edit-view").click();
  await expect(page.locator("#view-query")).toContainText("はい");
  await page.getByLabel("一覧名", { exact: true }).fill("My draft");
  await dialog(page).getByRole("checkbox", { name: "名称", exact: true }).uncheck();
  await page.locator("#confirm-view").click(); await expect(page.locator("#view-error")).toContainText("1つ以上");
  await dialog(page).getByRole("checkbox", { name: "名称", exact: true }).check();
  const remote = { name: "Other editor", columns: ["done", "name"], filter: { field: "done", value: false } };
  expect((await request.put(`/api/v1/apps/${app.id}/views/${view.id}`, { headers, data: { revision: 1, definition: remote } })).ok()).toBeTruthy();
  await page.locator("#confirm-view").click(); await expect(page.locator("#view-error")).toContainText("ほかの変更");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("My draft");
  await expect(dialog(page).getByRole("checkbox", { name: "完了", exact: true })).not.toBeChecked();
  const pending = gate(); const detail = `**/api/v1/apps/${app.id}/views/${view.id}`;
  await page.route(detail, async (route) => { pending.arrive(); await pending.wait; await route.continue(); });
  await page.locator("#reload-view").click(); await pending.arrival; await pendingLocked(page); pending.release();
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Other editor");
  await expect(page.getByLabel("一覧名", { exact: true })).toBeFocused(); await page.unroute(detail);
  await expect(dialog(page).getByRole("checkbox", { name: "完了", exact: true })).toBeChecked();
  await expect(page.locator("#view-query")).toContainText("いいえ");
  await page.getByLabel("一覧名", { exact: true }).fill("Updated"); await page.locator("#confirm-view").click();
  await expect(dialog(page)).not.toBeVisible();
  await expect(page.locator("#table-head th")).toHaveText(["完了", "名称", "操作"]);
  await expect(page.getByRole("cell", { name: "Pending", exact: true })).toBeVisible();
  const [latest] = await catalog(request, app); expect(latest.revision).toBe(3);
  await page.locator("#edit-view").click();
  expect((await request.put(`/api/v1/apps/${app.id}/views/${view.id}`, { headers, data: { revision: 3, definition: { ...latest.definition, name: "Changed before delete" } } })).ok()).toBeTruthy();
  page.once("dialog", (dialog) => dialog.accept()); await page.locator("#delete-view").click();
  await expect(page.locator("#view-error")).toContainText("ほかの変更");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Updated");
  await page.locator("#reload-view").click(); await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Changed before delete");
  page.once("dialog", (dialog) => dialog.accept()); await page.locator("#delete-view").click();
  await expect(dialog(page)).not.toBeVisible(); await expect(page.locator("#record-count")).toHaveText("このページに 2 件");
  expect(await catalog(request, app)).toEqual([]);
  expect((await (await request.get(`/api/v1/apps/${app.id}/records`)).json()).records).toHaveLength(2);
});

test("duplicate names retain column drafts and a different name can be saved", async ({ page, request }, info) => {
  const { app } = await seed(request, `View names ${info.project.name}`, [{ name: "Record" }]);
  await createView(request, app, { name: "Existing", columns: ["name"] });
  await page.goto("/"); await page.locator("#save-view").click();
  await page.getByLabel("一覧名", { exact: true }).fill("Existing");
  await dialog(page).getByRole("checkbox", { name: "メモ", exact: true }).uncheck();
  await page.locator("#confirm-view").click(); await expect(page.locator("#view-error")).toContainText("同じ名前");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Existing");
  await expect(dialog(page).getByRole("checkbox", { name: "メモ", exact: true })).not.toBeChecked();
  await expect(page.locator("#reload-view")).toBeHidden();
  await page.getByLabel("一覧名", { exact: true }).fill("Another"); await page.locator("#confirm-view").click();
  await expect(dialog(page)).not.toBeVisible(); expect(await catalog(request, app)).toHaveLength(2);
});

test("pending save and delete lock controls and Escape; failures allow retry with the same draft", async ({ page, request }, info) => {
  const { app } = await seed(request, `View pending ${info.project.name}`, [{ name: "Keep" }]);
  await page.goto("/"); await page.locator("#save-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("Retained");
  const path = `**/api/v1/apps/${app.id}/views`; const save = gate();
  await page.route(path, async (route) => { save.arrive(); await save.wait; await route.fulfill({ status: 503, json: { error: { code: "unavailable" } } }); });
  await page.locator("#confirm-view").click(); await save.arrival; await pendingLocked(page); save.release();
  await expect(page.locator("#view-error")).toContainText("少し待って再試行");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Retained");
  await page.unroute(path); await page.locator("#confirm-view").click(); await expect(dialog(page)).not.toBeVisible();
  const [view] = await catalog(request, app); await page.locator("#edit-view").click();
  const deletion = gate(); const detail = `**/api/v1/apps/${app.id}/views/${view.id}?*`;
  await page.route(detail, async (route) => { deletion.arrive(); await deletion.wait; await route.fulfill({ status: 503, json: { error: { code: "unavailable" } } }); });
  page.once("dialog", (dialog) => dialog.accept()); await page.locator("#delete-view").click();
  await deletion.arrival; await pendingLocked(page); deletion.release();
  await expect(page.locator("#view-error")).toContainText("少し待って再試行");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue("Retained");
  await page.unroute(detail); page.once("dialog", (dialog) => dialog.accept()); await page.locator("#delete-view").click();
  await expect(dialog(page)).not.toBeVisible(); await expect(page.getByRole("cell", { name: "Keep", exact: true })).toBeVisible();
});

test("catalog errors have a separate retry and catalog completion preserves record drafts and focus", async ({ page, request }, info) => {
  const { app } = await seed(request, `View catalog retry ${info.project.name}`, [{ name: "Before" }]);
  await createView(request, app, { name: "Saved", columns: ["name"] });
  const path = `**/api/v1/apps/${app.id}/views`;
  await page.route(path, (route) => route.fulfill({ status: 503, json: { error: { code: "unavailable" } } }));
  await page.goto("/"); await expect(page.locator("#view-status")).toContainText("レコードの操作は続けられます");
  await expect(page.locator("#export-csv")).toBeEnabled(); await expect(picker(page)).toBeDisabled();
  const invoking = page.getByRole("button", { name: /^レコード.*を編集$/ });
  await invoking.click(); await page.locator("#record-name").fill("Draft"); await page.keyboard.press("Escape"); await expect(invoking).toBeFocused();
  await page.unroute(path); const pending = gate();
  await page.route(path, async (route) => { pending.arrive(); await pending.wait; await route.continue(); });
  await page.locator("#reload-views").click(); await pending.arrival; await invoking.click(); await page.locator("#record-name").fill("Draft during retry");
  pending.release(); await expect(page.locator("#view-status")).toHaveText("");
  await expect(page.locator("#record-name")).toHaveValue("Draft during retry");
  await page.keyboard.press("Escape"); await expect(invoking).toBeFocused(); await expect(picker(page)).toBeEnabled();
});

test("a delayed catalog cannot replace another app's options", async ({ page, request }, info) => {
  const { app: first } = await seed(request, `Views A ${info.project.name}`, [{ name: "A" }]);
  const { app: second } = await seed(request, `Views B ${info.project.name}`, [{ name: "B" }]);
  await createView(request, first, { name: "Only A", columns: ["name"] });
  const b = await createView(request, second, { name: "Only B", columns: ["name"] });
  await page.goto("/"); await expect(picker(page)).toBeEnabled(); const pending = gate();
  const path = `**/api/v1/apps/${first.id}/views`;
  await page.route(path, async (route) => { pending.arrive(); await pending.wait; await route.continue(); });
  await page.getByRole("button", { name: first.name, exact: true }).click(); await pending.arrival;
  await page.getByRole("button", { name: second.name, exact: true }).click(); await expect(picker(page)).toBeEnabled();
  const finished = page.waitForResponse((response) => response.url().endsWith(`/apps/${first.id}/views`));
  pending.release(); await (await finished).finished();
  await expect(picker(page).locator("option")).toHaveText(["すべてのレコード（標準）", "Only B"]);
  await choose(page, b); await expect(page.getByRole("cell", { name: "B", exact: true })).toBeVisible();
});

test("refreshing a deleted selected view retains a newer applied search and control draft", async ({ page, request }, info) => {
  const { app } = await seed(request, `Deleted view query ${info.project.name}`, [{ name: "First" }, { name: "Second" }]);
  const view = await createView(request, app, { name: "Saved", columns: ["name"] });
  await page.goto("/"); await choose(page, view);
  expect((await request.delete(`/api/v1/apps/${app.id}/views/${view.id}?revision=1`, { headers })).status()).toBe(204);
  const pending = gate();
  await page.route(`**/api/v1/apps/${app.id}/views`, async (route) => { pending.arrive(); await pending.wait; await route.continue(); });
  await page.locator("#reload-views").click(); await pending.arrival;
  await page.locator("#search").fill("Second"); await page.getByRole("button", { name: "検索", exact: true }).click();
  await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
  await page.locator("#search").fill("Unsubmitted newer draft");
  pending.release(); await expect(page.locator("#view-status")).toHaveText("");
  await expect(page.locator("#search")).toHaveValue("Unsubmitted newer draft");
  await expect(page.locator("#query-summary")).toHaveText("適用中：文字検索「Second」");
  await expect(page.getByRole("cell", { name: "Second", exact: true })).toBeVisible();
  await expect(page.locator("#record-count")).toHaveText("このページに 1 件");
  await expect(picker(page)).toHaveValue(String(view.id));
  await expect(picker(page).locator("option:checked")).toContainText("削除済み");
  await expect(page.locator("#edit-view")).toBeDisabled();
  await page.locator("#save-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("Recovered");
  await expect(page.locator("#view-query")).toHaveText("文字検索「Second」");
  await page.locator("#confirm-view").click(); await expect(dialog(page)).not.toBeVisible();
  const [recovered] = await catalog(request, app); expect(recovered.definition.query).toBe("Second");
  expect(recovered.definition.columns).toEqual(["name"]);
  await choose(page, null); await expect(page.locator("#record-count")).toHaveText("このページに 2 件");
  await expect(page.locator("#search")).toHaveValue("");
});

test("refreshing a deleted selected view preserves a record dialog's draft and invoking focus", async ({ page, request }, info) => {
  const { app } = await seed(request, `Deleted view focus ${info.project.name}`, [{ name: "Record" }]);
  const view = await createView(request, app, { name: "Saved", columns: ["name"] });
  await page.goto("/"); await choose(page, view);
  expect((await request.delete(`/api/v1/apps/${app.id}/views/${view.id}?revision=1`, { headers })).status()).toBe(204);
  const pending = gate();
  await page.route(`**/api/v1/apps/${app.id}/views`, async (route) => { pending.arrive(); await pending.wait; await route.continue(); });
  await page.locator("#reload-views").click(); await pending.arrival;
  const invoking = page.getByRole("button", { name: /^レコード.*を編集$/ });
  await invoking.click(); await page.locator("#record-name").fill("Unsaved record draft");
  pending.release(); await expect(page.locator("#view-status")).toHaveText("");
  await expect(page.locator("#record-name")).toHaveValue("Unsaved record draft");
  await page.keyboard.press("Escape"); await expect(invoking).toBeFocused();
});

test("returning to the same app cannot let an older selection invalidate an open view editor", async ({ page, request }, info) => {
  const { app: first } = await seed(request, `View ABA A ${info.project.name}`, [{ name: "A" }]);
  const { app: second } = await seed(request, `View ABA B ${info.project.name}`, [{ name: "B" }]);
  await createView(request, first, { name: "Existing A", columns: ["name"] });
  await page.goto("/"); await expect(picker(page)).toBeEnabled();
  let calls = 0; const pending = gate();
  await page.route(`**/api/v1/apps/${first.id}/records?*`, async (route) => {
    if (++calls !== 1) return route.continue();
    pending.arrive(); await pending.wait; await route.continue();
  });
  await page.getByRole("button", { name: first.name, exact: true }).click(); await pending.arrival;
  await page.getByRole("button", { name: second.name, exact: true }).click(); await expect(picker(page)).toBeEnabled();
  await page.getByRole("button", { name: first.name, exact: true }).click(); await expect(picker(page)).toBeEnabled();
  await page.locator("#save-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("New after returning");
  const finished = page.waitForResponse((response) => response.url().includes(`/apps/${first.id}/records?`));
  pending.release(); await (await finished).finished();
  // Flush all responses initiated by the obsolete selection before saving.
  await page.waitForLoadState("networkidle");
  const saved = page.waitForResponse((response) => response.url().endsWith(`/apps/${first.id}/views`) && response.request().method() === "POST");
  await page.locator("#confirm-view").click(); expect((await saved).status()).toBe(201);
  await expect(dialog(page)).not.toBeVisible();
  expect((await catalog(request, first)).map((view) => view.definition.name)).toContain("New after returning");
});

test("changing columns preserves an unchanged API-created name with internal CR/LF", async ({ page, request }, info) => {
  const { app } = await seed(request, `View name line endings ${info.project.name}`, [{ name: "Record" }]);
  const originalName = "First\r\nSecond\rThird";
  const view = await createView(request, app, { name: originalName, columns: ["name", "done"] });
  await page.goto("/"); await choose(page, view); await page.locator("#edit-view").click();
  await dialog(page).getByRole("checkbox", { name: "完了", exact: true }).uncheck();
  await page.locator("#confirm-view").click(); await expect(dialog(page)).not.toBeVisible();
  const [updated] = await catalog(request, app);
  expect(updated.definition.name).toBe(originalName); expect(updated.definition.columns).toEqual(["name"]);
  await page.locator("#edit-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("Intentional rename");
  await page.locator("#confirm-view").click(); await expect(dialog(page)).not.toBeVisible();
  expect((await catalog(request, app))[0].definition.name).toBe("Intentional rename");
});

test("manual catalog retry supersedes startup loading without invalidating a later view editor", async ({ page, request }, info) => {
  const { app } = await seed(request, `Startup catalog ${info.project.name}`, [{ name: "Keep" }]);
  let calls = 0; const pending = gate();
  await page.route(`**/api/v1/apps/${app.id}/records?*`, async (route) => {
    if (++calls !== 1) return route.continue();
    pending.arrive(); await pending.wait; await route.continue();
  });
  await page.goto("/"); await pending.arrival;
  await page.locator("#reload-views").click(); await expect(picker(page)).toBeEnabled();
  await page.locator("#search").fill("Keep"); await page.getByRole("button", { name: "検索", exact: true }).click();
  await expect(page.locator("#save-view")).toBeEnabled();
  await page.locator("#save-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("Later editor");
  const finished = page.waitForResponse((response) => response.url().includes(`/apps/${app.id}/records?`));
  pending.release(); await (await finished).finished(); await page.waitForLoadState("networkidle");
  const saved = page.waitForResponse((response) => response.url().endsWith(`/apps/${app.id}/views`) && response.request().method() === "POST");
  await page.locator("#confirm-view").click(); expect((await saved).status()).toBe(201);
  await expect(dialog(page)).not.toBeVisible();
  expect((await catalog(request, app))[0].definition.query).toBe("Keep");
});

test("fast view selection discards earlier record responses", async ({ page, request }, info) => {
  const { app } = await seed(request, `Fast views ${info.project.name}`, [{ name: "Pending", done: false }, { name: "Finished", done: true }]);
  const a = await createView(request, app, { name: "Pending", columns: ["name"], filter: { field: "done", value: false } });
  const b = await createView(request, app, { name: "Finished", columns: ["done", "name"], filter: { field: "done", value: true } });
  await page.goto("/"); const pending = gate(); const path = `**/api/v1/apps/${app.id}/records?*`;
  await page.route(path, async (route) => {
    const filter = JSON.parse(new URL(route.request().url()).searchParams.get("filter") ?? "null");
    if (filter?.value !== false) return route.continue();
    pending.arrive(); await pending.wait; await route.continue();
  });
  await expect(picker(page)).toBeEnabled(); await picker(page).selectOption(String(a.id)); await pending.arrival;
  await choose(page, b); await expect(page.getByRole("cell", { name: "Finished", exact: true })).toBeVisible();
  const finished = page.waitForResponse((response) => response.url().includes(`/apps/${app.id}/records?`) && decodeURIComponent(response.url()).includes('"value":false'));
  pending.release(); await (await finished).finished();
  await expect(picker(page)).toHaveValue(String(b.id)); await expect(page.locator("#table-head th")).toHaveText(["完了", "名称", "操作"]);
  await expect(page.getByRole("cell", { name: "Finished", exact: true })).toBeVisible();
  await expect(page.locator("#query-summary")).toContainText("一致：はい");
});

test("API-created views keep literal names, typed zero/false/null, choice whitespace and multiline conditions", async ({ page, request }, info) => {
  const { app } = await seed(request, `Literal views ${info.project.name}`, [
    { name: "Target", amount: 0, done: false, due: "2024-02-29", status: "\uFEFFOpen", note: "a\r\nb\rc" },
    { name: "Empty", amount: 1, done: true, due: null, note: "" }, { name: "Missing" },
  ]);
  const markup = "<img src=x onerror=globalThis.__viewXss=1>";
  const definitions = [
    { name: markup, filter: { field: "amount", value: 0 } },
    { name: "False", filter: { field: "done", value: false } },
    { name: "Empty date", filter: { field: "due", value: null } },
    { name: "Choice", filter: { field: "status", value: "\uFEFFOpen" } },
    { name: "CRLF", filter: { field: "note", value: "a\r\nb\rc" } },
    { name: "Exact empty", filter: { field: "note", value: "" } },
  ];
  const views = [];
  for (const definition of definitions) views.push(await createView(request, app, { ...definition, columns: ["name"] }));
  await page.goto("/");
  for (const [index, view] of views.entries()) {
    await choose(page, view); await expect(page.locator("#record-count")).toHaveText(`このページに ${index === 2 ? 2 : 1} 件`);
    if (index !== 2) await expect(page.getByRole("cell", { name: index === 5 ? "Empty" : "Target", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "条件を適用", exact: true }).click();
    await expect(page.locator("#record-count")).toHaveText(`このページに ${index === 2 ? 2 : 1} 件`);
  }
  await choose(page, views[4]);
  await expect(page.getByLabel("比較する値", { exact: true })).toHaveValue("a\nb\nc");
  await page.locator("#save-view").click(); await page.getByLabel("一覧名", { exact: true }).fill("Copied CRLF"); await page.locator("#confirm-view").click();
  await expect(dialog(page)).not.toBeVisible();
  expect((await catalog(request, app)).find((view) => view.definition.name === "Copied CRLF").definition.filter.value).toBe("a\r\nb\rc");
  await choose(page, views[0]); await page.locator("#edit-view").focus(); await page.keyboard.press("Enter");
  await expect(page.getByLabel("一覧名", { exact: true })).toHaveValue(markup);
  await page.keyboard.press("Escape"); await expect(page.locator("#edit-view")).toBeFocused();
  expect(await page.locator("#business-apps-root img, #business-apps-root script").count()).toBe(0);
  expect(await page.evaluate(() => globalThis.__viewXss ?? null)).toBeNull();
});
