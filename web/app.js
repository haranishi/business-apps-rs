// SPDX-License-Identifier: Apache-2.0
const $ = (id) => document.getElementById(id);
const state = { apps: [], active: null, records: [], next: null, cursors: [null], page: 0, request: 0, loading: false };
let editingRecord = null;
let fieldCounter = 0;
let recordBusy = false;
let appBusy = false;

function setRecordBusy(busy) {
  recordBusy = busy;
  for (const input of $("record-form").querySelectorAll("input, button")) input.disabled = busy;
  $("save-record").textContent = busy ? "保存中…" : "保存";
}

function setAppBusy(busy) {
  appBusy = busy;
  for (const input of $("app-form").querySelectorAll("input, select, button")) input.disabled = busy;
  $("save-app").textContent = busy ? "作成中…" : "アプリを作成";
  if (!busy) refreshBuilder();
}

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

async function api(path, method = "GET", body) {
  const headers = {};
  if (method !== "GET") headers["X-Bizapp-Request"] = "1";
  if (body !== undefined) headers["Content-Type"] = "application/json";
  const response = await fetch(`/api/v1${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
  if (!response.ok) {
    const text = await response.text();
    let detail;
    try { detail = JSON.parse(text).error; } catch { /* Standard extractor rejection. */ }
    const message = response.status === 409 ? "ほかの変更が保存されています。最新版を読み込んでから、もう一度編集してください。" :
      response.status === 404 ? "このレコードまたはアプリは見つかりません。" :
      response.status >= 500 ? "保存先に接続できません。少し待って再試行してください。" : detail?.message || "入力内容を確認して再試行してください。";
    const error = new Error(message);
    error.status = response.status;
    throw error;
  }
  return response.status === 204 ? null : response.json();
}

function status(message) { $("status").textContent = message; }

function renderApps() {
  $("app-nav").replaceChildren(...state.apps.map((app) => {
    const button = element("button", app.name);
    if (app.id === state.active?.id) button.setAttribute("aria-current", "page");
    button.addEventListener("click", () => selectApp(app));
    return button;
  }));
}

async function selectApp(app) {
  state.active = app;
  state.records = [];
  state.next = null;
  state.cursors = [null]; state.page = 0;
  $("search").value = "";
  $("welcome").hidden = true;
  $("app-workspace").hidden = false;
  $("app-title").textContent = app.name;
  $("table-caption").textContent = `${app.name}のレコード一覧`;
  status(""); renderApps(); renderRecords();
  await loadRecords();
  document.querySelector(".table-scroll").scrollLeft = 0;
}

function renderRecords() {
  const tr = element("tr");
  for (const field of state.active.fields) tr.append(element("th", field.label));
  tr.append(element("th", "操作"));
  $("table-head").replaceChildren(tr);
  const rows = state.records.map((record) => {
    const row = element("tr");
    for (const field of state.active.fields) {
      const value = Object.hasOwn(record.values, field.key) ? record.values[field.key] : null;
      const text = value === null ? "—" : typeof value === "boolean" ? (value ? "はい" : "いいえ") : String(value);
      const cell = element("td"); cell.title = text;
      cell.append(element("span", text, "cell-value")); row.append(cell);
    }
    const action = element("td");
    const edit = element("button", "編集");
    edit.setAttribute("aria-label", `レコード${record.id}を編集`);
    edit.addEventListener("click", () => openRecord(record));
    action.append(edit); row.append(action);
    return row;
  });
  if (!rows.length) {
    const row = element("tr");
    const cell = element("td", $("search").value ? "一致するレコードが見つかりませんでした。" : "レコードはまだありません。「レコードを追加」から登録しましょう。", "empty-cell");
    cell.colSpan = state.active.fields.length + 1; row.append(cell); rows.push(row);
  }
  $("table-body").replaceChildren(...rows);
  $("record-count").textContent = state.loading ? "読み込み中…" : `このページに ${state.records.length} 件`;
  $("previous-page").disabled = state.loading || state.page === 0;
  $("next-page").disabled = state.loading || state.next === null;
  $("page-number").textContent = `${state.page + 1} ページ`;
}

async function loadRecords() {
  const request = ++state.request;
  const appId = state.active.id;
  const params = new URLSearchParams({ limit: "30" });
  const cursor = state.cursors[state.page];
  if (cursor !== null) params.set("before", String(cursor));
  const query = $("search").value.trim();
  if (query) params.set("q", query);
  state.loading = true; renderRecords();
  try {
    const page = await api(`/apps/${appId}/records?${params}`);
    if (request !== state.request) return null;
    state.records = page.records; state.next = page.next_cursor;
    status(""); return true;
  } catch (error) {
    if (request !== state.request) return null;
    status(error.message);
    return false;
  } finally {
    if (request === state.request) { state.loading = false; renderRecords(); }
  }
}

function addField(label = "", kind = "text", required = false) {
  if ($("field-builder").children.length >= 32) return;
  const key = `field_${++fieldCounter}`;
  const row = element("div", undefined, "field-row"); row.dataset.key = key;
  const nameLabel = element("label", "項目名"); const name = element("input");
  name.required = true; name.maxLength = 80; name.value = label; name.placeholder = "例：顧客名"; name.className = "field-label"; nameLabel.append(name);
  const kindBlock = element("div"); const kindLabel = element("label", "種類"); const select = element("select"); select.className = "field-kind";
  select.id = `kind-${key}`; kindLabel.htmlFor = select.id;
  for (const [value, text] of [["text", "テキスト"], ["number", "数値"], ["boolean", "チェック"]]) {
    const option = element("option", text); option.value = value; select.append(option);
  }
  select.value = kind; kindBlock.append(kindLabel, select);
  const requiredLabel = element("label", undefined, "checkbox-label"); const checkbox = element("input");
  checkbox.type = "checkbox"; checkbox.checked = required; checkbox.className = "field-required";
  requiredLabel.append(checkbox, element("span", "必須"));
  const remove = element("button", "×", "remove"); remove.type = "button"; remove.setAttribute("aria-label", "項目を削除");
  remove.addEventListener("click", () => { row.remove(); refreshBuilder(); });
  row.append(nameLabel, kindBlock, requiredLabel, remove); $("field-builder").append(row); refreshBuilder();
}

function refreshBuilder() {
  const count = $("field-builder").children.length;
  $("add-field").disabled = count >= 32;
  for (const button of $("field-builder").querySelectorAll(".remove")) button.disabled = count <= 1;
}

function openApp(template) {
  if (appBusy) return;
  $("app-form").reset(); $("app-error").textContent = ""; $("field-builder").replaceChildren(); fieldCounter = 0;
  $("save-app").disabled = false;
  $("app-name").value = template?.name || "";
  const fields = template?.fields || [["名称", "text", true], ["メモ", "text", false]];
  for (const field of fields) addField(...field);
  $("app-dialog").showModal(); $("app-name").focus();
}

function openRecord(record = null) {
  if (recordBusy) return;
  editingRecord = record;
  $("record-form").reset(); $("record-error").textContent = "";
  $("record-dialog-title").textContent = record ? "レコードを編集" : "レコードを追加";
  $("delete-record").hidden = !record; $("reload-record").hidden = true;
  $("save-record").disabled = false; $("delete-record").disabled = false;
  const inputs = state.active.fields.map((field) => {
    const block = element("div", undefined, "record-field");
    const label = element("label", field.label); const input = element("input");
    input.id = `record-${field.key}`; input.dataset.key = field.key; input.dataset.kind = field.kind;
    const value = record && Object.hasOwn(record.values, field.key) ? record.values[field.key] : null;
    if (field.kind === "boolean") { input.type = "checkbox"; input.checked = value === true; label.className = "checkbox-label"; label.prepend(input); }
    else {
      input.type = field.kind === "number" ? "number" : "text";
      if (field.kind === "number") { input.step = "any"; input.min = String(-Number.MAX_SAFE_INTEGER); input.max = String(Number.MAX_SAFE_INTEGER); }
      else input.maxLength = 4000;
      input.required = field.required; input.value = value ?? ""; label.htmlFor = input.id;
    }
    if (field.required) label.append(element("span", "必須", "required-tag"));
    block.append(label); if (field.kind !== "boolean") block.append(input);
    return block;
  });
  $("record-fields").replaceChildren(...inputs);
  if (!$("record-dialog").open) $("record-dialog").showModal();
  $("record-fields").querySelector("input")?.focus();
}

function recordValues() {
  const values = Object.create(null);
  for (const input of $("record-fields").querySelectorAll("input")) {
    values[input.dataset.key] = input.dataset.kind === "boolean" ? input.checked :
      input.dataset.kind === "number" ? (input.value === "" ? null : Number(input.value)) : input.value;
  }
  return values;
}

async function refreshAfterSave(message) {
  $("record-dialog").close(); $("new-record").focus();
  state.cursors = [null]; state.page = 0;
  const refreshed = await loadRecords();
  if (refreshed) { status(message); document.querySelector(".table-scroll").scrollLeft = 0; }
  else if (refreshed === false) status(`${message} 一覧を更新できませんでした。再読み込みしてください。`);
}

$("create-app").addEventListener("click", () => openApp());
$("first-app").addEventListener("click", () => openApp());
$("customer-template").addEventListener("click", () => openApp({ name: "顧客管理", fields: [["顧客名", "text", true], ["連絡先", "text", false], ["対応済み", "boolean", false]] }));
$("inventory-template").addEventListener("click", () => openApp({ name: "在庫管理", fields: [["商品名", "text", true], ["数量", "number", true], ["メモ", "text", false]] }));
$("add-field").addEventListener("click", () => addField());
for (const button of document.querySelectorAll(".close-app")) button.addEventListener("click", () => { if (!appBusy) $("app-dialog").close(); });
for (const button of document.querySelectorAll(".close-record")) button.addEventListener("click", () => { if (!recordBusy) $("record-dialog").close(); });
$("new-record").addEventListener("click", () => openRecord());
$("record-dialog").addEventListener("cancel", (event) => { if (recordBusy) event.preventDefault(); });
$("app-dialog").addEventListener("cancel", (event) => { if (appBusy) event.preventDefault(); });

$("app-form").addEventListener("submit", async (event) => {
  event.preventDefault(); if (appBusy) return;
  setAppBusy(true); $("app-error").textContent = "";
  const fields = Array.from($("field-builder").children, (row) => ({ key: row.dataset.key, label: row.querySelector(".field-label").value.trim(), kind: row.querySelector(".field-kind").value, required: row.querySelector(".field-required").checked }));
  try {
    const app = await api("/apps", "POST", { name: $("app-name").value.trim(), fields });
    state.apps.unshift(app); $("app-dialog").close();
    await selectApp(app); status("アプリを作成しました。レコードを追加できます。"); $("new-record").focus();
  } catch (error) { $("app-error").textContent = error.message; }
  finally { setAppBusy(false); }
});

$("search-form").addEventListener("submit", async (event) => { event.preventDefault(); state.cursors = [null]; state.page = 0; await loadRecords(); });
$("clear-search").addEventListener("click", async () => { $("search").value = ""; state.cursors = [null]; state.page = 0; await loadRecords(); });
$("next-page").addEventListener("click", async () => {
  if (state.loading || state.next === null) return;
  state.cursors[state.page + 1] = state.next; state.page++;
  if (await loadRecords() === false) { state.page--; renderRecords(); }
});
$("previous-page").addEventListener("click", async () => {
  if (state.loading || state.page === 0) return;
  state.page--; if (await loadRecords() === false) { state.page++; renderRecords(); }
});

$("record-form").addEventListener("submit", async (event) => {
  event.preventDefault(); if (recordBusy) return;
  setRecordBusy(true); $("record-error").textContent = "";
  const path = `/apps/${state.active.id}/records`;
  try {
    const values = recordValues();
    await api(editingRecord ? `${path}/${editingRecord.id}` : path, editingRecord ? "PUT" : "POST", editingRecord ? { revision: editingRecord.revision, values } : { values });
    await refreshAfterSave("レコードを保存しました。");
  } catch (error) { $("record-error").textContent = error.message; $("reload-record").hidden = error.status !== 409; }
  finally { setRecordBusy(false); }
});

$("reload-record").addEventListener("click", async () => {
  if (recordBusy) return;
  setRecordBusy(true);
  try { const latest = await api(`/apps/${state.active.id}/records/${editingRecord.id}`); setRecordBusy(false); openRecord(latest); }
  catch (error) { $("record-error").textContent = error.message; }
  finally { setRecordBusy(false); }
});
$("delete-record").addEventListener("click", async () => {
  if (recordBusy || !editingRecord || !confirm("このレコードを削除します。よろしいですか？")) return;
  setRecordBusy(true);
  try { await api(`/apps/${state.active.id}/records/${editingRecord.id}?revision=${editingRecord.revision}`, "DELETE"); await refreshAfterSave("レコードを削除しました。"); }
  catch (error) { $("record-error").textContent = error.message; $("reload-record").hidden = error.status !== 409; }
  finally { setRecordBusy(false); }
});

try {
  state.apps = await api("/apps"); renderApps();
  if (state.apps.length) await selectApp(state.apps[0]);
} catch { status("アプリを読み込めませんでした。サーバーの起動を確認して、ページを再読み込みしてください。"); }

const tableContainer = document.querySelector(".table-scroll");
new ResizeObserver(() => {
  $("table-hint").hidden = tableContainer.scrollWidth <= tableContainer.clientWidth + 1;
}).observe(tableContainer);
