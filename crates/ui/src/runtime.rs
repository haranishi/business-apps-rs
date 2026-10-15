// SPDX-License-Identifier: Apache-2.0
use crate::{dom::*, layout};
use business_apps_domain::{App, Record, RecordFilter};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Event, EventTarget, HtmlDialogElement, RequestInit, ResizeObserver, Response};

#[derive(Default)]
pub struct State {
    pub apps: Vec<App>,
    pub active: Option<App>,
    pub records: Vec<Record>,
    pub next: Option<i64>,
    pub cursors: Vec<Option<i64>>,
    pub page: usize,
    pub request: u64,
    pub loading: bool,
    pub query: String,
    pub filter: Option<RecordFilter>,
    pub export_ready: bool,
    pub exporting: bool,
    pub editing_record: Option<Record>,
    pub editing_app: Option<App>,
    pub app_form_request: u64,
    pub field_counter: u32,
    pub record_busy: bool,
    pub app_busy: bool,
}

pub struct Ui {
    pub state: RefCell<State>,
    listeners: RefCell<Vec<Listener>>,
    resize: RefCell<Option<TableObserver>>,
}

struct TableObserver {
    observer: ResizeObserver,
    _callback: Closure<dyn FnMut()>,
}

impl Drop for TableObserver {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

// A fixed set of document listeners belongs to the UI. Replacing record/field
// nodes never creates leaked closures; each listener unregisters when dropped.
struct Listener {
    target: EventTarget,
    kind: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.kind,
            self.callback.as_ref().unchecked_ref(),
            true,
        );
    }
}

thread_local! {
    static APPLICATION: RefCell<Option<Rc<Ui>>> = const { RefCell::new(None) };
}

pub fn start() -> Result {
    let ui = Rc::new(Ui {
        state: RefCell::new(State::default()),
        listeners: RefCell::new(Vec::new()),
        resize: RefCell::new(None),
    });
    let installed = (|| {
        layout::mount()?;
        ui.install()?;
        APPLICATION.with(|slot| *slot.borrow_mut() = Some(ui.clone()));
        document()
            .body()
            .expect("browser body")
            .set_attribute("data-ui-runtime", "rust-wasm")?;
        Ok(())
    })();
    if let Err(error) = installed {
        APPLICATION.with(|slot| slot.borrow_mut().take());
        if let Some(root) = document().get_element_by_id("business-apps-root") {
            root.remove();
        }
        if let Some(body) = document().body() {
            let _ = body.remove_attribute("data-ui-runtime");
        }
        // Dropping `ui` also unregisters every listener installed before failure.
        // The loader can now display its error through the retained boot message.
        return Err(error);
    }
    if let Some(boot) = document().get_element_by_id("boot-status") {
        boot.remove();
    }
    spawn_local(async move {
        ui.initialize().await;
    });
    Ok(())
}

impl Ui {
    fn install(self: &Rc<Self>) -> Result {
        for kind in ["click", "submit", "change", "cancel"] {
            let weak = Rc::downgrade(self);
            let callback = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                let Some(target) = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                if kind == "cancel" {
                    let state = ui.state.borrow();
                    if (target.id() == "record-dialog" && state.record_busy)
                        || (target.id() == "app-dialog" && state.app_busy)
                    {
                        event.prevent_default();
                    }
                    return;
                }
                if kind == "change" {
                    if target.id() == "filter-field" {
                        if let Err(error) = ui.render_filter_value() {
                            ui.unexpected(error);
                        }
                    } else if target.id() == "filter-mode" {
                        ui.update_filter_mode();
                    }
                    if target.class_list_contains("field-kind")
                        && let Some(row) = target.closest(".field-row").ok().flatten()
                    {
                        ui.update_choices(&row);
                    }
                    return;
                }
                if kind == "submit" {
                    if !["app-form", "record-form", "search-form"].contains(&target.id().as_str()) {
                        return;
                    }
                    event.prevent_default();
                    spawn_local(async move {
                        if let Err(error) = ui.submit(&target.id()).await {
                            ui.unexpected(error);
                        }
                    });
                } else if let Some(button) = target.closest("button").ok().flatten() {
                    spawn_local(async move {
                        if let Err(error) = ui.click(&button).await {
                            ui.unexpected(error);
                        }
                    });
                }
            });
            let target: EventTarget = document().unchecked_into();
            target.add_event_listener_with_callback_and_bool(
                kind,
                callback.as_ref().unchecked_ref(),
                true,
            )?;
            self.listeners.borrow_mut().push(Listener {
                target,
                kind,
                callback,
            });
        }
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(ui) = weak.upgrade() {
                ui.update_table_hint();
            }
        });
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref())?;
        observer.observe(&query(&id("app-workspace"), ".table-scroll"));
        *self.resize.borrow_mut() = Some(TableObserver {
            observer,
            _callback: callback,
        });
        Ok(())
    }

    async fn initialize(&self) {
        let result = async {
            let apps: Vec<App> = api("/apps", "GET", None)
                .await
                .map_err(|error| JsValue::from_str(&error.message))?;
            let first = apps.first().cloned();
            self.state.borrow_mut().apps = apps;
            self.render_apps()?;
            disabled(&id("create-app"), false);
            if let Some(first) = first {
                self.select_app(first).await?;
            } else {
                hidden(&id("welcome"), false);
                text("status", "");
            }
            Ok::<(), JsValue>(())
        }
        .await;
        if result.is_err() {
            disabled(&id("create-app"), false);
            text(
                "status",
                "アプリを読み込めませんでした。サーバーの起動を確認して、ページを再読み込みしてください。",
            );
        }
    }

    fn unexpected(&self, _error: JsValue) {
        text(
            "status",
            "操作を完了できませんでした。入力を確認して、もう一度お試しください。",
        );
    }

    pub fn dialog(name: &str) -> HtmlDialogElement {
        id(name).unchecked_into()
    }

    async fn submit(&self, target: &str) -> Result {
        match target {
            "app-form" => self.save_app().await,
            "record-form" => self.save_record().await,
            "search-form" => {
                let filter = match self.read_filter() {
                    Ok(filter) => filter,
                    Err(message) => {
                        text("status", message);
                        return Ok(());
                    }
                };
                {
                    let mut state = self.state.borrow_mut();
                    state.query = value(&id("search")).trim().to_owned();
                    state.filter = filter;
                }
                self.reset_paging();
                self.load_records().await.map(|_| ())
            }
            _ => Ok(()),
        }
    }

    async fn click(&self, button: &web_sys::Element) -> Result {
        if button.has_attribute("disabled") {
            return Ok(());
        }
        let id_string = button.id();
        match id_string.as_str() {
            "create-app" | "first-app" => self.open_app(None, None),
            "customer-template" => self.open_template(true),
            "inventory-template" => self.open_template(false),
            "add-field" => {
                self.add_field("", business_apps_domain::FieldKind::Text, false, &[], None)
            }
            "new-record" => self.open_record(None),
            "edit-app" => self.edit_app().await,
            "reload-app" => self.reload_app().await,
            "reload-record" => self.reload_record().await,
            "delete-record" => self.delete_record().await,
            "export-csv" => self.export_csv().await,
            "clear-search" => {
                set_value(&id("search"), "");
                self.state.borrow_mut().query.clear();
                self.state.borrow_mut().filter = None;
                self.reset_filter_controls()?;
                self.reset_paging();
                self.load_records().await.map(|_| ())
            }
            "clear-filter" => {
                self.state.borrow_mut().filter = None;
                self.reset_filter_controls()?;
                self.reset_paging();
                self.load_records().await.map(|_| ())
            }
            "next-page" => self.turn_page(true).await,
            "previous-page" => self.turn_page(false).await,
            _ if button.class_list_contains("close-app") => {
                if !self.state.borrow().app_busy {
                    Self::dialog("app-dialog").close();
                }
                Ok(())
            }
            _ if button.class_list_contains("close-record") => {
                if !self.state.borrow().record_busy {
                    Self::dialog("record-dialog").close();
                }
                Ok(())
            }
            _ if button.has_attribute("data-app-id") => {
                let app_id = button
                    .get_attribute("data-app-id")
                    .and_then(|value| value.parse::<i64>().ok());
                let app = self
                    .state
                    .borrow()
                    .apps
                    .iter()
                    .find(|app| Some(app.id) == app_id)
                    .cloned();
                if let Some(app) = app {
                    self.select_app(app).await?;
                }
                Ok(())
            }
            _ if button.has_attribute("data-record-id") => {
                let record_id = button
                    .get_attribute("data-record-id")
                    .and_then(|value| value.parse::<i64>().ok());
                let record = self
                    .state
                    .borrow()
                    .records
                    .iter()
                    .find(|record| Some(record.id) == record_id)
                    .cloned();
                self.open_record(record)
            }
            _ if button.has_attribute("data-field-action") => self.field_action(button),
            _ => Ok(()),
        }
    }

    pub fn reset_paging(&self) {
        let mut state = self.state.borrow_mut();
        state.cursors = vec![None];
        state.page = 0;
    }

    pub fn update_table_hint(&self) {
        let table = query(&id("app-workspace"), ".table-scroll");
        hidden(
            &id("table-hint"),
            table.scroll_width() <= table.client_width() + 1,
        );
    }
}

// Element.classList would add another generated binding just for membership.
trait Classes {
    fn class_list_contains(&self, class: &str) -> bool;
}
impl Classes for web_sys::Element {
    fn class_list_contains(&self, class: &str) -> bool {
        self.class_name()
            .split_ascii_whitespace()
            .any(|name| name == class)
    }
}

pub struct ApiError {
    pub status: u16,
    pub message: String,
}
impl From<JsValue> for ApiError {
    fn from(_error: JsValue) -> Self {
        Self {
            status: 0,
            message: "サーバーに接続できません。接続を確認して再試行してください。".into(),
        }
    }
}

pub async fn api<T: DeserializeOwned>(
    path: &str,
    method: &str,
    body: Option<Value>,
) -> std::result::Result<T, ApiError> {
    let response = api_response(path, method, body).await?;
    let status = response.status();
    let raw = JsFuture::from(response.text()?)
        .await?
        .as_string()
        .unwrap_or_default();
    let data = if status == 204 {
        Value::Null
    } else {
        serde_json::from_str(&raw).map_err(|_| ApiError {
            status,
            message: "サーバーからの応答を読み込めませんでした。".into(),
        })?
    };
    serde_json::from_value(data).map_err(|_| ApiError {
        status,
        message: "サーバーからの応答を読み込めませんでした。".into(),
    })
}

pub async fn api_response(
    path: &str,
    method: &str,
    body: Option<Value>,
) -> std::result::Result<Response, ApiError> {
    let options = RequestInit::new();
    options.set_method(method);
    let headers = web_sys::Headers::new()?;
    if method != "GET" {
        headers.set("X-Bizapp-Request", "1")?;
    }
    if let Some(body) = body {
        headers.set("Content-Type", "application/json")?;
        options.set_body(&JsValue::from_str(&body.to_string()));
    }
    options.set_headers(&headers);
    let response = JsFuture::from(
        web_sys::window()
            .expect("browser window")
            .fetch_with_str_and_init(&format!("/api/v1{path}"), &options),
    )
    .await?
    .dyn_into::<Response>()?;
    let status = response.status();
    if !response.ok() {
        let raw = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        let data: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let detail = data.pointer("/error/message").and_then(Value::as_str);
        let validation = detail.map(|message| {
            if message.starts_with("Select fields need") {
                "選択肢は1〜50件、1件80文字以内で、重複しないように入力してください。"
            } else {
                message
            }
        });
        let message = match status {
            409 => {
                "ほかの変更が保存されています。最新版を読み込んでから、もう一度編集してください。"
            }
            404 => "このレコードまたはアプリは見つかりません。",
            413 if data.pointer("/error/code").and_then(Value::as_str) == Some("export_limit") => {
                "CSVは最大10,000件・16 MiBまでです。検索で絞り込んでから再試行してください。"
            }
            500.. => "保存先に接続できません。少し待って再試行してください。",
            _ => validation.unwrap_or("入力内容を確認して再試行してください。"),
        };
        return Err(ApiError {
            status,
            message: message.into(),
        });
    }
    Ok(response)
}

#[derive(Deserialize)]
pub struct RecordPage {
    pub records: Vec<Record>,
    pub next_cursor: Option<i64>,
}
