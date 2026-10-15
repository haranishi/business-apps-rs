// SPDX-License-Identifier: Apache-2.0
use crate::{dom::*, runtime::Ui};
use business_apps_domain::{FieldKind, RecordFilter, validate_record_filter};
use serde_json::Value;

pub fn query_parameters(query: &str, filter: Option<&RecordFilter>) -> String {
    let mut parameters = format!("q={}", String::from(js_sys::encode_uri_component(query)));
    if let Some(filter) = filter {
        let json = serde_json::to_string(filter).expect("scalar filter serializes");
        parameters.push_str("&filter=");
        parameters.push_str(&String::from(js_sys::encode_uri_component(&json)));
    }
    parameters
}

impl Ui {
    pub fn reset_filter_controls(&self) -> Result {
        let select = id("filter-field");
        clear(&select);
        child(&select, "option", &[("value", "")], Some("条件なし"))?;
        if let Some(app) = &self.state.borrow().active {
            for field in &app.fields {
                child(
                    &select,
                    "option",
                    &[("value", &field.key)],
                    Some(&field.label),
                )?;
            }
        }
        set_value(&id("filter-mode"), "equals");
        self.render_filter_value()
    }

    pub fn render_filter_value(&self) -> Result {
        let selected = value(&id("filter-field"));
        let field = self
            .state
            .borrow()
            .active
            .as_ref()
            .and_then(|app| app.fields.iter().find(|field| field.key == selected))
            .cloned();
        let host = id("filter-value-host");
        clear(&host);
        let kind = field.as_ref().map(|field| field.kind);
        let control = child(
            &host,
            if matches!(kind, Some(FieldKind::Boolean | FieldKind::Select)) {
                "select"
            } else if kind == Some(FieldKind::Textarea) {
                "textarea"
            } else {
                "input"
            },
            &[("id", "filter-value")],
            None,
        )?;
        match kind {
            Some(FieldKind::Boolean) => {
                child(
                    &control,
                    "option",
                    &[("value", "")],
                    Some("選択してください"),
                )?;
                child(&control, "option", &[("value", "true")], Some("はい"))?;
                child(&control, "option", &[("value", "false")], Some("いいえ"))?;
            }
            Some(FieldKind::Select) => {
                child(
                    &control,
                    "option",
                    &[("value", "")],
                    Some("選択してください"),
                )?;
                for option in &field.expect("selected field").options {
                    child(&control, "option", &[("value", option)], Some(option))?;
                }
            }
            Some(FieldKind::Number) => {
                for (name, value) in [
                    ("type", "number"),
                    ("step", "any"),
                    ("min", "-9007199254740991"),
                    ("max", "9007199254740991"),
                ] {
                    control.set_attribute(name, value)?;
                }
            }
            Some(FieldKind::Date) => {
                for (name, value) in [
                    ("type", "date"),
                    ("min", "0001-01-01"),
                    ("max", "9999-12-31"),
                ] {
                    control.set_attribute(name, value)?;
                }
            }
            Some(FieldKind::Textarea) => {
                control.set_attribute("rows", "3")?;
                control.set_attribute("maxlength", "256")?;
            }
            _ => {
                control.set_attribute("type", "text")?;
                control.set_attribute("maxlength", "256")?;
            }
        }
        self.update_filter_mode();
        Ok(())
    }

    pub fn update_filter_mode(&self) {
        let selected = !value(&id("filter-field")).is_empty();
        disabled(&id("filter-mode"), !selected);
        let needs_value = selected && value(&id("filter-mode")) == "equals";
        disabled(&id("filter-value"), !needs_value);
        let selected = value(&id("filter-field"));
        let text_field = self
            .state
            .borrow()
            .active
            .as_ref()
            .and_then(|app| app.fields.iter().find(|field| field.key == selected))
            .is_some_and(|field| matches!(field.kind, FieldKind::Text | FieldKind::Textarea));
        required(&id("filter-value"), needs_value && !text_field);
    }

    pub fn read_filter(&self) -> std::result::Result<Option<RecordFilter>, &'static str> {
        let key = value(&id("filter-field"));
        if key.is_empty() {
            return Ok(None);
        }
        let state = self.state.borrow();
        let app = state.active.as_ref().ok_or("アプリを選択してください。")?;
        let field = app
            .fields
            .iter()
            .find(|field| field.key == key)
            .ok_or("絞り込む項目を選び直してください。")?;
        let raw = value(&id("filter-value"));
        let value = if value(&id("filter-mode")) == "empty" {
            Value::Null
        } else {
            match field.kind {
                FieldKind::Boolean => match raw.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => return Err("比較する値を選択してください。"),
                },
                FieldKind::Number => raw
                    .parse::<f64>()
                    .ok()
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .ok_or("比較する数値を確認してください。")?,
                _ => Value::String(raw),
            }
        };
        let filter = RecordFilter { field: key, value };
        validate_record_filter(&app.fields, &filter)
            .map_err(|_| "絞り込み条件を確認してください。文字列は128文字までです。")?;
        Ok(Some(filter))
    }

    pub fn render_query_summary(&self) {
        let state = self.state.borrow();
        if !state.loading && !state.records_ready {
            text(
                "query-summary",
                "一覧の取得に失敗しました。条件を再適用してください。",
            );
            return;
        }
        let mut parts = Vec::new();
        if !state.query.is_empty() {
            parts.push(format!("文字検索「{}」", state.query));
        }
        if let (Some(app), Some(filter)) = (&state.active, &state.filter)
            && let Some(field) = app.fields.iter().find(|field| field.key == filter.field)
        {
            let condition = match &filter.value {
                Value::Null => "空欄".into(),
                Value::Bool(value) => format!("一致：{}", if *value { "はい" } else { "いいえ" }),
                Value::String(value) => format!("一致：{value}"),
                value => format!("一致：{value}"),
            };
            parts.push(format!("{}（{}）", field.label, condition));
        }
        let description = if parts.is_empty() {
            "すべてのレコード".into()
        } else {
            parts.join(" ／ ")
        };
        text(
            "query-summary",
            &format!(
                "{}：{description}",
                if state.loading {
                    "読み込み中"
                } else {
                    "適用中"
                }
            ),
        );
    }
}
