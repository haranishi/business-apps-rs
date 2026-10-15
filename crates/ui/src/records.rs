// SPDX-License-Identifier: Apache-2.0
use crate::{
    dom::*,
    merge_edited_values,
    runtime::{RecordPage, Ui, api},
};
use business_apps_domain::{App, FieldKind, Record, Values};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use web_sys::HtmlFormElement;

impl Ui {
    pub fn render_apps(&self) -> Result {
        let state = self.state.borrow();
        let nav = id("app-nav");
        clear(&nav);
        for app in &state.apps {
            let button = child(
                &nav,
                "button",
                &[("data-app-id", &app.id.to_string())],
                Some(&app.name),
            )?;
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.id == app.id)
            {
                button.set_attribute("aria-current", "page")?;
            }
        }
        Ok(())
    }

    pub async fn select_app(&self, app: App) -> Result {
        {
            let mut state = self.state.borrow_mut();
            state.app_form_request += 1;
            state.active = Some(app.clone());
            state.records.clear();
            state.next = None;
            state.cursors = vec![None];
            state.page = 0;
            state.query.clear();
            state.filter = None;
            state.export_ready = false;
            state.loading = true;
        }
        set_value(&id("search"), "");
        self.reset_filter_controls()?;
        hidden(&id("welcome"), true);
        hidden(&id("app-workspace"), false);
        text("app-title", &app.name);
        text("table-caption", &format!("{}のレコード一覧", app.name));
        text("status", "");
        self.render_apps()?;
        self.render_records()?;
        self.load_records().await?;
        query(&id("app-workspace"), ".table-scroll").set_scroll_left(0);
        Ok(())
    }

    pub fn render_records(&self) -> Result {
        let state = self.state.borrow();
        let Some(app) = &state.active else {
            return Ok(());
        };
        let head = id("table-head");
        clear(&head);
        let row = child(&head, "tr", &[], None)?;
        for field in &app.fields {
            child(&row, "th", &[], Some(&field.label))?;
        }
        child(&row, "th", &[], Some("操作"))?;
        let body = id("table-body");
        clear(&body);
        for record in &state.records {
            let row = child(&body, "tr", &[], None)?;
            for field in &app.fields {
                let display = display_value(record.values.get(&field.key));
                let cell = child(&row, "td", &[("title", &display)], None)?;
                child(&cell, "span", &[("class", "cell-value")], Some(&display))?;
            }
            let action = child(&row, "td", &[], None)?;
            child(
                &action,
                "button",
                &[
                    ("data-record-id", &record.id.to_string()),
                    ("aria-label", &format!("レコード{}を編集", record.id)),
                ],
                Some("編集"),
            )?;
        }
        if state.records.is_empty() {
            let row = child(&body, "tr", &[], None)?;
            let message = if state.query.is_empty() && state.filter.is_none() {
                "レコードはまだありません。「レコードを追加」から登録しましょう。"
            } else {
                "一致するレコードが見つかりませんでした。"
            };
            child(
                &row,
                "td",
                &[
                    ("class", "empty-cell"),
                    ("colspan", &(app.fields.len() + 1).to_string()),
                ],
                Some(message),
            )?;
        }
        text(
            "record-count",
            &if state.loading {
                "読み込み中…".into()
            } else {
                format!("このページに {} 件", state.records.len())
            },
        );
        disabled(&id("previous-page"), state.loading || state.page == 0);
        disabled(&id("next-page"), state.loading || state.next.is_none());
        text("page-number", &format!("{} ページ", state.page + 1));
        disabled(
            &id("export-csv"),
            state.loading || !state.export_ready || state.exporting,
        );
        self.update_table_hint();
        self.render_query_summary();
        Ok(())
    }

    /// None means a newer request superseded this response. Callers must not
    /// roll back a page cursor belonging to another app or a newer search.
    pub async fn load_records(&self) -> Result<Option<bool>> {
        let (request, path) = {
            let mut state = self.state.borrow_mut();
            let Some(app) = &state.active else {
                return Ok(None);
            };
            let mut path = format!("/apps/{}/records?limit=30", app.id);
            if let Some(Some(cursor)) = state.cursors.get(state.page) {
                path.push_str(&format!("&before={cursor}"));
            }
            path.push('&');
            path.push_str(&crate::filters::query_parameters(
                &state.query,
                state.filter.as_ref(),
            ));
            state.request += 1;
            state.loading = true;
            state.export_ready = false;
            (state.request, path)
        };
        text("export-status", "");
        self.render_records()?;
        let response = api::<RecordPage>(&path, "GET", None).await;
        if request != self.state.borrow().request {
            return Ok(None);
        }
        let success = match response {
            Ok(page) => {
                let mut state = self.state.borrow_mut();
                state.records = page.records;
                state.next = page.next_cursor;
                state.export_ready = true;
                text("status", "");
                true
            }
            Err(error) => {
                text("status", &error.message);
                false
            }
        };
        self.state.borrow_mut().loading = false;
        self.render_records()?;
        Ok(Some(success))
    }

    pub async fn turn_page(&self, next: bool) -> Result {
        {
            let mut state = self.state.borrow_mut();
            if state.loading {
                return Ok(());
            }
            if next {
                let Some(cursor) = state.next else {
                    return Ok(());
                };
                let page = state.page + 1;
                if state.cursors.len() <= page {
                    state.cursors.resize(page + 1, None);
                }
                state.cursors[page] = Some(cursor);
                state.page = page;
            } else {
                if state.page == 0 {
                    return Ok(());
                }
                state.page -= 1;
            }
        }
        if self.load_records().await? == Some(false) {
            {
                let mut state = self.state.borrow_mut();
                if next {
                    state.page -= 1;
                } else {
                    state.page += 1;
                }
            }
            self.render_records()?;
        }
        Ok(())
    }

    pub fn set_record_busy(&self, busy: bool) {
        self.state.borrow_mut().record_busy = busy;
        for control in all(&id("record-form"), "input, select, textarea, button") {
            disabled(&control, busy);
        }
        text("save-record", if busy { "保存中…" } else { "保存" });
    }

    pub fn open_record(&self, record: Option<Record>) -> Result {
        let app = {
            let mut state = self.state.borrow_mut();
            if state.record_busy {
                return Ok(());
            }
            state.app_form_request += 1;
            state.editing_record = record.clone();
            let Some(app) = &state.active else {
                return Ok(());
            };
            app.clone()
        };
        id("record-form").unchecked_ref::<HtmlFormElement>().reset();
        text("record-error", "");
        text(
            "record-dialog-title",
            if record.is_some() {
                "レコードを編集"
            } else {
                "レコードを追加"
            },
        );
        hidden(&id("delete-record"), record.is_none());
        hidden(&id("reload-record"), true);
        disabled(&id("save-record"), false);
        disabled(&id("delete-record"), false);
        let fields = id("record-fields");
        clear(&fields);
        for field in &app.fields {
            let block = child(&fields, "div", &[("class", "record-field")], None)?;
            let label = child(&block, "label", &[], Some(&field.label))?;
            let tag = match field.kind {
                FieldKind::Textarea => "textarea",
                FieldKind::Select => "select",
                _ => "input",
            };
            let input = element(tag, None, None)?;
            let input_id = format!("record-{}", field.key);
            input.set_attribute("id", &input_id)?;
            input.set_attribute("data-key", &field.key)?;
            input.set_attribute("data-kind", kind_name(field.kind))?;
            let original = record
                .as_ref()
                .and_then(|record| record.values.get(&field.key));
            if field.kind == FieldKind::Boolean {
                input.set_attribute("type", "checkbox")?;
                set_checked(&input, original == Some(&Value::Bool(true)));
                label.set_class_name("checkbox-label");
                label.insert_before(&input, label.first_child().as_ref())?;
            } else {
                match field.kind {
                    FieldKind::Select => {
                        child(&input, "option", &[("value", "")], Some("選択してください"))?;
                        for option in &field.options {
                            child(&input, "option", &[("value", option)], Some(option))?;
                        }
                    }
                    FieldKind::Textarea => {
                        input.set_attribute("rows", "5")?;
                        input.set_attribute("maxlength", "8000")?;
                    }
                    FieldKind::Number => {
                        for (key, value) in [
                            ("type", "number"),
                            ("step", "any"),
                            ("min", "-9007199254740991"),
                            ("max", "9007199254740991"),
                        ] {
                            input.set_attribute(key, value)?;
                        }
                    }
                    FieldKind::Date => {
                        for (key, value) in [
                            ("type", "date"),
                            ("min", "0001-01-01"),
                            ("max", "9999-12-31"),
                        ] {
                            input.set_attribute(key, value)?;
                        }
                    }
                    _ => {
                        input.set_attribute("type", "text")?;
                        input.set_attribute("maxlength", "4000")?;
                    }
                }
                required(&input, field.required);
                set_value(&input, &input_value(original));
                label.set_attribute("for", &input_id)?;
                append(&block, &input)?;
            }
            if field.required {
                child(&label, "span", &[("class", "required-tag")], Some("必須"))?;
            }
        }
        let dialog = Self::dialog("record-dialog");
        if !dialog.open() {
            dialog.show_modal()?;
        }
        if let Some(input) = fields.query_selector("input, select, textarea")? {
            focus(&input);
        }
        Ok(())
    }

    fn record_values(&self) -> Values {
        let controls = all(&id("record-fields"), "input, select, textarea");
        let edits: Vec<(String, FieldKind, String, bool)> = controls
            .iter()
            .map(|control| {
                let key = control.get_attribute("data-key").expect("record field key");
                let kind = kind(
                    &control
                        .get_attribute("data-kind")
                        .expect("record field kind"),
                );
                let checked = kind == FieldKind::Boolean && checked(control);
                (key, kind, value(control), checked)
            })
            .collect();
        let state = self.state.borrow();
        merge_edited_values(
            state.editing_record.as_ref().map(|record| &record.values),
            edits
                .iter()
                .map(|(key, kind, value, checked)| (key.as_str(), *kind, value.clone(), *checked)),
        )
    }

    async fn refresh_after_save(&self, message: &str) -> Result {
        Self::dialog("record-dialog").close();
        focus(&id("new-record"));
        self.reset_paging();
        match self.load_records().await? {
            Some(true) => {
                text("status", message);
                query(&id("app-workspace"), ".table-scroll").set_scroll_left(0);
            }
            Some(false) => text(
                "status",
                &format!("{message} 一覧を更新できませんでした。再読み込みしてください。"),
            ),
            None => {}
        }
        Ok(())
    }

    pub async fn save_record(&self) -> Result {
        let (app_id, record) = {
            let state = self.state.borrow();
            if state.record_busy {
                return Ok(());
            }
            let Some(app) = &state.active else {
                return Ok(());
            };
            (app.id, state.editing_record.clone())
        };
        self.set_record_busy(true);
        text("record-error", "");
        let values = self.record_values();
        let mut path = format!("/apps/{app_id}/records");
        let body = if let Some(record) = &record {
            path.push_str(&format!("/{}", record.id));
            json!({"revision":record.revision,"values":values})
        } else {
            json!({"values":values})
        };
        let result = match api::<Value>(
            &path,
            if record.is_some() { "PUT" } else { "POST" },
            Some(body),
        )
        .await
        {
            Ok(_) => self.refresh_after_save("レコードを保存しました。").await,
            Err(error) => {
                text("record-error", &error.message);
                hidden(&id("reload-record"), error.status != 409);
                Ok(())
            }
        };
        self.set_record_busy(false);
        result
    }

    pub async fn reload_record(&self) -> Result {
        let (app_id, record_id) = {
            let state = self.state.borrow();
            if state.record_busy {
                return Ok(());
            }
            let (Some(app), Some(record)) = (&state.active, &state.editing_record) else {
                return Ok(());
            };
            (app.id, record.id)
        };
        self.set_record_busy(true);
        let result = match api::<Record>(
            &format!("/apps/{app_id}/records/{record_id}"),
            "GET",
            None,
        )
        .await
        {
            Ok(record) => {
                self.set_record_busy(false);
                self.open_record(Some(record))
            }
            Err(error) => {
                text("record-error", &error.message);
                Ok(())
            }
        };
        self.set_record_busy(false);
        result
    }

    pub async fn delete_record(&self) -> Result {
        let (app_id, record) = {
            let state = self.state.borrow();
            if state.record_busy {
                return Ok(());
            }
            let (Some(app), Some(record)) = (&state.active, &state.editing_record) else {
                return Ok(());
            };
            (app.id, record.clone())
        };
        if !web_sys::window()
            .expect("browser window")
            .confirm_with_message("このレコードを削除します。よろしいですか？")?
        {
            return Ok(());
        }
        self.set_record_busy(true);
        let result = match api::<Value>(
            &format!(
                "/apps/{app_id}/records/{}?revision={}",
                record.id, record.revision
            ),
            "DELETE",
            None,
        )
        .await
        {
            Ok(_) => self.refresh_after_save("レコードを削除しました。").await,
            Err(error) => {
                text("record-error", &error.message);
                hidden(&id("reload-record"), error.status != 409);
                Ok(())
            }
        };
        self.set_record_busy(false);
        result
    }
}

fn input_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number
            .as_f64()
            .map_or_else(|| number.to_string(), |number| number.to_string()),
        Some(value) => value.to_string(),
    }
}

fn display_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "—".into(),
        Some(Value::Bool(true)) => "はい".into(),
        Some(Value::Bool(false)) => "いいえ".into(),
        value => input_value(value),
    }
}
