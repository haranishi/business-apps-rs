// SPDX-License-Identifier: Apache-2.0
use crate::{
    dom::*,
    filters::query_description,
    runtime::{Ui, api},
    views::ViewEditor,
};
use business_apps_domain::{App, SavedView, ViewDefinition, validate_view};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use web_sys::HtmlFormElement;

impl Ui {
    pub fn open_view_editor(&self, edit: bool) -> Result {
        let (app, editor) = {
            let mut state = self.state.borrow_mut();
            if state.views.busy || !state.records_ready || !state.views.loaded {
                return Ok(());
            }
            let Some(app) = state.active.clone() else {
                return Ok(());
            };
            let prior = if edit {
                state.views.selected.clone()
            } else {
                None
            };
            if edit && prior.is_none() {
                return Ok(());
            }
            let definition = ViewDefinition {
                name: prior
                    .as_ref()
                    .map(|view| view.definition.name.clone())
                    .unwrap_or_default(),
                query: state.query.clone(),
                filter: state.filter.clone(),
                columns: state
                    .views
                    .selected
                    .as_ref()
                    .map(|view| view.definition.columns.clone())
                    .unwrap_or_else(|| app.fields.iter().map(|field| field.key.clone()).collect()),
            };
            let editor = ViewEditor {
                app_id: app.id,
                scope: state.views.request,
                prior,
                definition,
            };
            state.views.editor_request += 1;
            state.views.editor = Some(editor.clone());
            (app, editor)
        };
        self.render_view_editor(&app, &editor)?;
        Self::dialog("view-dialog").show_modal()?;
        focus(&id("view-name"));
        Ok(())
    }

    fn render_view_editor(&self, app: &App, editor: &ViewEditor) -> Result {
        id("view-form").unchecked_ref::<HtmlFormElement>().reset();
        text(
            "view-dialog-title",
            if editor.prior.is_some() {
                "一覧の設定"
            } else {
                "一覧を保存"
            },
        );
        set_value(&id("view-name"), &editor.definition.name);
        text(
            "view-query",
            &query_description(
                app,
                &editor.definition.query,
                editor.definition.filter.as_ref(),
            ),
        );
        text("view-error", "");
        hidden(&id("reload-view"), true);
        hidden(&id("delete-view"), editor.prior.is_none());
        let host = id("view-columns");
        clear(&host);
        let columns = &editor.definition.columns;
        let fields = columns
            .iter()
            .filter_map(|key| app.fields.iter().find(|field| &field.key == key))
            .chain(
                app.fields
                    .iter()
                    .filter(|field| !columns.contains(&field.key)),
            );
        for field in fields {
            let row = child(&host, "div", &[("class", "view-column")], None)?;
            let label = child(&row, "label", &[], None)?;
            let check = child(
                &label,
                "input",
                &[("type", "checkbox"), ("data-view-key", &field.key)],
                None,
            )?;
            set_checked(&check, columns.contains(&field.key));
            child(&label, "span", &[], Some(&field.label))?;
            for (action, title) in [("up", "上へ"), ("down", "下へ")] {
                child(
                    &row,
                    "button",
                    &[
                        ("type", "button"),
                        ("data-view-column-action", action),
                        ("aria-label", &format!("{}を{title}", field.label)),
                    ],
                    Some(title),
                )?;
            }
        }
        self.set_view_busy(false);
        Ok(())
    }

    fn update_view_column_controls(&self) {
        let rows = children(&id("view-columns"));
        let busy = self.state.borrow().views.busy;
        for (index, row) in rows.iter().enumerate() {
            disabled(
                &query(row, "[data-view-column-action=up]"),
                busy || index == 0,
            );
            disabled(
                &query(row, "[data-view-column-action=down]"),
                busy || index + 1 == rows.len(),
            );
        }
    }

    pub fn view_column_action(&self, button: &web_sys::Element) -> Result {
        if self.state.borrow().views.busy {
            return Ok(());
        }
        let Some(row) = button.closest(".view-column")? else {
            return Ok(());
        };
        let host = id("view-columns");
        let rows = children(&host);
        let Some(index) = rows.iter().position(|item| item == &row) else {
            return Ok(());
        };
        match button.get_attribute("data-view-column-action").as_deref() {
            Some("up") if index > 0 => {
                host.insert_before(&row, Some(&rows[index - 1]))?;
            }
            Some("down") if index + 1 < rows.len() => {
                host.insert_before(&rows[index + 1], Some(&row))?;
            }
            _ => return Ok(()),
        }
        self.update_view_column_controls();
        focus(&query(&row, "input"));
        Ok(())
    }

    fn set_view_busy(&self, busy: bool) {
        self.state.borrow_mut().views.busy = busy;
        for control in all(&id("view-form"), "input, button") {
            disabled(&control, busy);
        }
        let editing = self
            .state
            .borrow()
            .views
            .editor
            .as_ref()
            .is_some_and(|editor| editor.prior.is_some());
        text(
            "confirm-view",
            if busy {
                "処理中…"
            } else if editing {
                "変更を保存"
            } else {
                "一覧を保存"
            },
        );
        self.update_view_column_controls();
        self.update_view_controls();
    }

    // Both the app/catalog generation and the editor generation must still
    // match before an async response may replace a draft or the active view.
    fn view_editor_current(&self, editor: &ViewEditor, request: u64) -> bool {
        let state = self.state.borrow();
        state.views.editor_request == request
            && state.views.request == editor.scope
            && state.active.as_ref().map(|app| app.id) == Some(editor.app_id)
    }

    fn view_operation(&self) -> Option<(ViewEditor, u64)> {
        let state = self.state.borrow();
        if state.views.busy {
            return None;
        }
        Some((state.views.editor.clone()?, state.views.editor_request))
    }

    pub async fn save_view(&self) -> Result {
        let Some((editor, request)) = self.view_operation() else {
            return Ok(());
        };
        let mut definition = editor.definition.clone();
        let name = value(&id("view-name"));
        // Single-line inputs strip CR/LF from valid API-origin names. A column
        // edit must preserve the original name when its display is unchanged.
        definition.name = if name == definition.name.replace(['\r', '\n'], "") {
            definition.name.clone()
        } else {
            name.trim().to_owned()
        };
        definition.columns = all(&id("view-columns"), "input")
            .into_iter()
            .filter(checked)
            .filter_map(|check| check.get_attribute("data-view-key"))
            .collect();
        let valid = self
            .state
            .borrow()
            .active
            .as_ref()
            .is_some_and(|app| validate_view(&app.fields, &definition).is_ok());
        if !valid {
            text(
                "view-error",
                "一覧名を1〜80文字で入力し、表示する項目を1つ以上選んでください。",
            );
            return Ok(());
        }
        self.set_view_busy(true);
        text("view-error", "");
        let base = format!("/apps/{}/views", editor.app_id);
        let (path, method, body) = if let Some(prior) = &editor.prior {
            (
                format!("{base}/{}", prior.id),
                "PUT",
                json!({"revision":prior.revision,"definition":definition}),
            )
        } else {
            (base, "POST", json!(definition))
        };
        let result = api::<SavedView>(&path, method, Some(body)).await;
        if !self.view_editor_current(&editor, request) {
            return Ok(());
        }
        self.set_view_busy(false);
        match result {
            Ok(view) => {
                self.replace_view(&view);
                Self::dialog("view-dialog").close();
                self.apply_view(Some(view)).await?;
            }
            Err(error) => {
                text("view-error", &error.message);
                hidden(
                    &id("reload-view"),
                    error.status != 409
                        || error.code.as_deref() == Some("duplicate_view_name")
                        || editor.prior.is_none(),
                );
            }
        }
        Ok(())
    }

    fn replace_view(&self, view: &SavedView) {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .views
            .list
            .iter_mut()
            .find(|entry| entry.id == view.id)
        {
            *entry = view.clone();
        } else {
            state.views.list.insert(0, view.clone());
        }
    }

    pub async fn reload_view(&self) -> Result {
        let Some((mut editor, request)) = self.view_operation() else {
            return Ok(());
        };
        let Some(prior) = &editor.prior else {
            return Ok(());
        };
        self.set_view_busy(true);
        let result = api::<SavedView>(
            &format!("/apps/{}/views/{}", editor.app_id, prior.id),
            "GET",
            None,
        )
        .await;
        if !self.view_editor_current(&editor, request) {
            return Ok(());
        }
        self.set_view_busy(false);
        match result {
            Ok(view) => {
                self.replace_view(&view);
                editor.definition = view.definition.clone();
                editor.prior = Some(view);
                let app = {
                    let mut state = self.state.borrow_mut();
                    state.views.editor = Some(editor.clone());
                    state.active.clone().expect("current app")
                };
                self.render_view_editor(&app, &editor)?;
                focus(&id("view-name"));
            }
            Err(error) => text("view-error", &error.message),
        }
        Ok(())
    }

    pub async fn delete_view(&self) -> Result {
        let Some((editor, request)) = self.view_operation() else {
            return Ok(());
        };
        let Some(prior) = &editor.prior else {
            return Ok(());
        };
        if !web_sys::window()
            .expect("browser window")
            .confirm_with_message(
                "この一覧を削除します。レコードは削除されません。よろしいですか？",
            )?
        {
            return Ok(());
        }
        self.set_view_busy(true);
        text("view-error", "");
        let result = api::<Value>(
            &format!(
                "/apps/{}/views/{}?revision={}",
                editor.app_id, prior.id, prior.revision
            ),
            "DELETE",
            None,
        )
        .await;
        if !self.view_editor_current(&editor, request) {
            return Ok(());
        }
        self.set_view_busy(false);
        match result {
            Ok(_) => {
                self.state
                    .borrow_mut()
                    .views
                    .list
                    .retain(|view| view.id != prior.id);
                Self::dialog("view-dialog").close();
                self.apply_view(None).await?;
            }
            Err(error) => {
                text("view-error", &error.message);
                hidden(&id("reload-view"), error.status != 409);
            }
        }
        Ok(())
    }
}
