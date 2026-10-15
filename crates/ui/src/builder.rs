// SPDX-License-Identifier: Apache-2.0
use crate::{
    dom::*,
    runtime::{Ui, api},
};
use business_apps_domain::{App, Field, FieldKind, NewApp};
use serde_json::json;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlFormElement, HtmlTextAreaElement};

impl Ui {
    pub fn set_app_busy(&self, busy: bool) {
        self.state.borrow_mut().app_busy = busy;
        for control in all(&id("app-form"), "input, select, textarea, button") {
            disabled(&control, busy);
        }
        let updating = self.state.borrow().editing_app.is_some();
        text(
            "save-app",
            if busy {
                "保存中…"
            } else if updating {
                "変更を保存"
            } else {
                "アプリを作成"
            },
        );
        if !busy {
            self.refresh_builder();
        }
    }

    pub fn add_field(
        &self,
        label: &str,
        kind: FieldKind,
        required_flag: bool,
        options: &[String],
        stored_key: Option<&str>,
    ) -> Result {
        if id("field-builder").children().length() >= 32 {
            return Ok(());
        }
        let key = if let Some(key) = stored_key {
            key.into()
        } else {
            loop {
                let mut state = self.state.borrow_mut();
                state.field_counter += 1;
                let key = format!("field_{}", state.field_counter);
                let used_in_dom = children(&id("field-builder"))
                    .iter()
                    .any(|row| row.get_attribute("data-key").as_deref() == Some(&key));
                let used_in_schema = state
                    .editing_app
                    .as_ref()
                    .is_some_and(|app| app.fields.iter().any(|field| field.key == key));
                if !used_in_dom && !used_in_schema {
                    break key;
                }
            }
        };
        let row = child(
            &id("field-builder"),
            "div",
            &[
                ("class", "field-row"),
                ("data-key", &key),
                (
                    "data-existing",
                    if stored_key.is_some() {
                        "true"
                    } else {
                        "false"
                    },
                ),
            ],
            None,
        )?;
        let name_label = child(&row, "label", &[], Some("項目名"))?;
        let name = child(
            &name_label,
            "input",
            &[
                ("required", ""),
                ("maxlength", "80"),
                ("placeholder", "例：顧客名"),
                ("class", "field-label"),
            ],
            None,
        )?;
        set_value(&name, label);
        let kind_block = child(&row, "div", &[], None)?;
        let kind_id = format!("kind-{key}");
        child(&kind_block, "label", &[("for", &kind_id)], Some("種類"))?;
        let select = child(
            &kind_block,
            "select",
            &[("id", &kind_id), ("class", "field-kind")],
            None,
        )?;
        for (value, label) in [
            ("text", "テキスト"),
            ("textarea", "複数行テキスト"),
            ("number", "数値"),
            ("boolean", "チェック"),
            ("date", "日付"),
            ("select", "選択肢"),
        ] {
            child(&select, "option", &[("value", value)], Some(label))?;
        }
        set_value(&select, kind_name(kind));
        let required_label = child(&row, "label", &[("class", "checkbox-label")], None)?;
        let checkbox = child(
            &required_label,
            "input",
            &[("type", "checkbox"), ("class", "field-required")],
            None,
        )?;
        set_checked(&checkbox, required_flag);
        child(&required_label, "span", &[], Some("必須"))?;
        let choices_label = child(
            &row,
            "label",
            &[("class", "field-options-label")],
            Some("選択肢（1行に1つ）"),
        )?;
        let choices = child(
            &choices_label,
            "textarea",
            &[
                ("class", "field-options"),
                ("rows", "3"),
                ("maxlength", "8049"),
                ("placeholder", "未対応\n対応中\n完了"),
            ],
            None,
        )?;
        set_value(&choices, &options.join("\n"));
        let actions = child(&row, "div", &[("class", "field-actions")], None)?;
        for (class, action, label, text) in [
            ("move-up", "up", "項目を上へ移動", "↑ 上へ"),
            ("move-down", "down", "項目を下へ移動", "↓ 下へ"),
            ("remove", "remove", "項目を削除", "×"),
        ] {
            child(
                &actions,
                "button",
                &[
                    ("type", "button"),
                    ("class", class),
                    ("data-field-action", action),
                    ("aria-label", label),
                ],
                Some(text),
            )?;
        }
        self.update_choices(&row);
        self.refresh_builder();
        Ok(())
    }

    pub fn update_choices(&self, row: &Element) {
        let visible = value(&query(row, ".field-kind")) == "select";
        hidden(&query(row, ".field-options-label"), !visible);
        let choices = query(row, ".field-options");
        required(&choices, visible);
        disabled(&choices, !visible);
    }

    fn refresh_builder(&self) {
        let rows = children(&id("field-builder"));
        let updating = self.state.borrow().editing_app.is_some();
        disabled(&id("add-field"), rows.len() >= 32);
        for (index, row) in rows.iter().enumerate() {
            let existing = row.get_attribute("data-existing").as_deref() == Some("true");
            disabled(&query(row, ".field-kind"), existing);
            let required_control = query(row, ".field-required");
            disabled(&required_control, updating);
            if updating && !existing {
                set_checked(&required_control, false);
            }
            query(row, ".field-options")
                .unchecked_ref::<HtmlTextAreaElement>()
                .set_read_only(existing);
            disabled(
                &query(row, ".field-options"),
                value(&query(row, ".field-kind")) != "select",
            );
            disabled(&query(row, ".remove"), rows.len() <= 1 || existing);
            disabled(&query(row, ".move-up"), index == 0);
            disabled(&query(row, ".move-down"), index + 1 == rows.len());
        }
    }

    pub fn field_action(&self, button: &Element) -> Result {
        if self.state.borrow().app_busy {
            return Ok(());
        }
        let Some(row) = button.closest(".field-row")? else {
            return Ok(());
        };
        let builder = id("field-builder");
        match button.get_attribute("data-field-action").as_deref() {
            Some("remove") => {
                let next = row
                    .next_element_sibling()
                    .or_else(|| row.previous_element_sibling());
                row.remove();
                self.refresh_builder();
                if let Some(next) = next {
                    focus(&query(&next, ".field-label"));
                }
            }
            Some("up") => {
                if let Some(prior) = row.previous_element_sibling() {
                    builder.insert_before(&row, Some(&prior))?;
                }
                self.refresh_builder();
                focus(&query(&row, ".field-label"));
            }
            Some("down") => {
                if let Some(next) = row.next_element_sibling() {
                    builder.insert_before(&row, next.next_sibling().as_ref())?;
                }
                self.refresh_builder();
                focus(&query(&row, ".field-label"));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn open_template(&self, customer: bool) -> Result {
        let (name, definitions) = if customer {
            (
                "顧客管理",
                vec![
                    ("顧客名", FieldKind::Text, true),
                    ("連絡先", FieldKind::Text, false),
                    ("対応済み", FieldKind::Boolean, false),
                ],
            )
        } else {
            (
                "在庫管理",
                vec![
                    ("商品名", FieldKind::Text, true),
                    ("数量", FieldKind::Number, true),
                    ("メモ", FieldKind::Textarea, false),
                ],
            )
        };
        let fields = definitions
            .into_iter()
            .map(|(label, kind, required)| Field {
                key: String::new(),
                label: label.into(),
                kind,
                required,
                options: vec![],
            })
            .collect();
        self.open_app(
            Some(NewApp {
                name: name.into(),
                fields,
            }),
            None,
        )
    }

    pub fn open_app(&self, template: Option<NewApp>, app: Option<App>) -> Result {
        if self.state.borrow().app_busy {
            return Ok(());
        }
        {
            let mut state = self.state.borrow_mut();
            state.app_form_request += 1;
            state.editing_app = app.clone();
            state.field_counter = 0;
        }
        id("app-form").unchecked_ref::<HtmlFormElement>().reset();
        text("app-error", "");
        clear(&id("field-builder"));
        disabled(&id("save-app"), false);
        let name = app
            .as_ref()
            .map(|app| app.name.as_str())
            .or_else(|| template.as_ref().map(|template| template.name.as_str()))
            .unwrap_or("");
        set_value(&id("app-name"), name);
        text(
            "app-dialog-title",
            if app.is_some() {
                "フォーム設定"
            } else {
                "アプリを作る"
            },
        );
        query(&id("app-dialog"), ".close-app[aria-label]").set_attribute(
            "aria-label",
            if app.is_some() {
                "フォーム設定を閉じる"
            } else {
                "アプリ作成を閉じる"
            },
        )?;
        hidden(&id("app-settings-hint"), app.is_none());
        hidden(&id("reload-app"), true);
        text(
            "save-app",
            if app.is_some() {
                "変更を保存"
            } else {
                "アプリを作成"
            },
        );
        if let Some(app) = app {
            for field in app.fields {
                self.add_field(
                    &field.label,
                    field.kind,
                    field.required,
                    &field.options,
                    Some(&field.key),
                )?;
            }
        } else if let Some(template) = template {
            for field in template.fields {
                self.add_field(
                    &field.label,
                    field.kind,
                    field.required,
                    &field.options,
                    None,
                )?;
            }
        } else {
            self.add_field("名称", FieldKind::Text, true, &[], None)?;
            self.add_field("メモ", FieldKind::Textarea, false, &[], None)?;
        }
        let dialog = Self::dialog("app-dialog");
        if !dialog.open() {
            dialog.show_modal()?;
        }
        focus(&id("app-name"));
        Ok(())
    }

    fn app_input(&self) -> NewApp {
        let state = self.state.borrow();
        let fields = children(&id("field-builder"))
            .iter()
            .map(|row| {
                let key = row.get_attribute("data-key").expect("builder field key");
                let kind = kind(&value(&query(row, ".field-kind")));
                let prior = state
                    .editing_app
                    .as_ref()
                    .and_then(|app| app.fields.iter().find(|field| field.key == key));
                let options = if kind != FieldKind::Select {
                    vec![]
                } else if let Some(prior) = prior {
                    prior.options.clone()
                } else {
                    value(&query(row, ".field-options"))
                        .split('\n')
                        .map(trim_ui)
                        .filter(|option| !option.is_empty())
                        .map(String::from)
                        .collect()
                };
                Field {
                    key,
                    label: trim_ui(&value(&query(row, ".field-label"))).into(),
                    kind,
                    required: checked(&query(row, ".field-required")),
                    options,
                }
            })
            .collect();
        NewApp {
            name: trim_ui(&value(&id("app-name"))).into(),
            fields,
        }
    }

    pub async fn save_app(&self) -> Result {
        if self.state.borrow().app_busy {
            return Ok(());
        }
        self.set_app_busy(true);
        text("app-error", "");
        let input = self.app_input();
        let prior = self.state.borrow().editing_app.clone();
        let mut body = serde_json::to_value(input).expect("serializable app schema");
        if let Some(prior) = &prior {
            body["revision"] = json!(prior.revision);
        }
        let path = prior
            .as_ref()
            .map_or("/apps".into(), |prior| format!("/apps/{}", prior.id));
        let result = match api::<App>(
            &path,
            if prior.is_some() { "PUT" } else { "POST" },
            Some(body),
        )
        .await
        {
            Ok(app) => {
                {
                    let mut state = self.state.borrow_mut();
                    if prior.is_some() {
                        for existing in &mut state.apps {
                            if existing.id == app.id {
                                *existing = app.clone();
                            }
                        }
                    } else {
                        state.apps.insert(0, app.clone());
                    }
                }
                Self::dialog("app-dialog").close();
                let result = self.select_app(app).await;
                text(
                    "status",
                    if prior.is_some() {
                        "フォームを更新しました。既存のレコードは保持されています。"
                    } else {
                        "アプリを作成しました。レコードを追加できます。"
                    },
                );
                focus(&id(if prior.is_some() {
                    "edit-app"
                } else {
                    "new-record"
                }));
                result
            }
            Err(error) => {
                text("app-error", &error.message);
                hidden(&id("reload-app"), error.status != 409);
                Ok(())
            }
        };
        self.set_app_busy(false);
        result
    }

    pub async fn edit_app(&self) -> Result {
        let (app_id, request) = {
            let mut state = self.state.borrow_mut();
            if state.app_busy {
                return Ok(());
            }
            let Some(active) = &state.active else {
                return Ok(());
            };
            let app_id = active.id;
            state.app_form_request += 1;
            (app_id, state.app_form_request)
        };
        disabled(&id("edit-app"), true);
        let mut result = Ok(());
        match api::<App>(&format!("/apps/{app_id}"), "GET", None).await {
            Ok(app) => {
                let valid = {
                    let state = self.state.borrow();
                    request == state.app_form_request
                        && state
                            .active
                            .as_ref()
                            .is_some_and(|active| active.id == app_id)
                };
                if valid
                    && !Self::dialog("app-dialog").open()
                    && !Self::dialog("record-dialog").open()
                {
                    disabled(&id("edit-app"), false);
                    focus(&id("edit-app"));
                    result = self.open_app(None, Some(app));
                }
            }
            Err(error) => text("status", &error.message),
        }
        disabled(&id("edit-app"), false);
        result
    }

    pub async fn reload_app(&self) -> Result {
        let prior = {
            let state = self.state.borrow();
            if state.app_busy {
                return Ok(());
            }
            let Some(prior) = &state.editing_app else {
                return Ok(());
            };
            prior.clone()
        };
        self.set_app_busy(true);
        let result = match api::<App>(&format!("/apps/{}", prior.id), "GET", None).await {
            Ok(app) => {
                self.set_app_busy(false);
                self.open_app(None, Some(app))
            }
            Err(error) => {
                text("app-error", &error.message);
                Ok(())
            }
        };
        self.set_app_busy(false);
        result
    }
}

fn trim_ui(value: &str) -> &str {
    value.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}
