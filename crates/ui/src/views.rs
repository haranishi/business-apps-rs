// SPDX-License-Identifier: Apache-2.0
use crate::{
    dom::*,
    runtime::{Ui, api},
};
use business_apps_domain::{App, Field, MAX_VIEWS_PER_APP, SavedView, ViewDefinition};

#[derive(Default)]
pub struct ViewState {
    pub list: Vec<SavedView>,
    pub selected: Option<SavedView>,
    pub loaded: bool,
    pub loading: bool,
    pub request: u64,
    pub editor: Option<ViewEditor>,
    pub editor_request: u64,
    pub busy: bool,
}

#[derive(Clone)]
pub struct ViewEditor {
    pub app_id: i64,
    pub selection: u64,
    pub prior: Option<SavedView>,
    pub definition: ViewDefinition,
}

pub fn visible_fields<'a>(app: &'a App, selected: Option<&SavedView>) -> Vec<&'a Field> {
    selected.map_or_else(
        || app.fields.iter().collect(),
        |view| {
            view.definition
                .columns
                .iter()
                .filter_map(|key| app.fields.iter().find(|field| &field.key == key))
                .collect()
        },
    )
}

impl Ui {
    pub fn render_views(&self) -> Result {
        let state = self.state.borrow();
        let select = id("record-view");
        clear(&select);
        child(
            &select,
            "option",
            &[("value", "")],
            Some("すべてのレコード（標準）"),
        )?;
        for view in &state.views.list {
            child(
                &select,
                "option",
                &[("value", &view.id.to_string())],
                Some(&view.definition.name),
            )?;
        }
        if let Some(selected) = &state.views.selected
            && !state.views.list.iter().any(|view| view.id == selected.id)
        {
            child(
                &select,
                "option",
                &[("value", &selected.id.to_string()), ("disabled", "")],
                Some(&format!(
                    "{}（削除済み・現在の表示）",
                    selected.definition.name
                )),
            )?;
        }
        set_value(
            &select,
            &state
                .views
                .selected
                .as_ref()
                .map(|view| view.id.to_string())
                .unwrap_or_default(),
        );
        text(
            "view-count",
            &format!(
                "保存した一覧 {}/{}",
                state.views.list.len(),
                MAX_VIEWS_PER_APP
            ),
        );
        drop(state);
        self.update_view_controls();
        Ok(())
    }

    pub fn update_view_controls(&self) {
        let state = self.state.borrow();
        let available = state.views.loaded && !state.views.loading && !state.views.busy;
        let selected_exists = state
            .views
            .selected
            .as_ref()
            .is_some_and(|selected| state.views.list.iter().any(|view| view.id == selected.id));
        disabled(&id("record-view"), !available);
        disabled(&id("reload-views"), state.views.loading || state.views.busy);
        disabled(
            &id("save-view"),
            !available || !state.records_ready || state.views.list.len() >= MAX_VIEWS_PER_APP,
        );
        disabled(
            &id("edit-view"),
            !available || !state.records_ready || !selected_exists,
        );
        let message = if let Some(view) = &state.views.selected {
            if state.views.loaded && !selected_exists {
                "この一覧は削除されています。現在の表示は保持しています。標準の一覧に切り替えるか、別名で保存できます。".into()
            } else if view.definition.query != state.query || view.definition.filter != state.filter
            {
                "条件を変更しています。「一覧の設定」で保存すると更新できます。".into()
            } else {
                format!(
                    "「{}」の条件と表示項目を使用しています。",
                    view.definition.name
                )
            }
        } else if !state.query.is_empty() || state.filter.is_some() {
            "現在の条件は保存されていません。「この条件で一覧を保存」で登録できます。".into()
        } else {
            String::new()
        };
        text("view-modified", &message);
    }

    pub async fn load_views(&self, app_id: i64) -> Result {
        let request = {
            let mut state = self.state.borrow_mut();
            if state.active.as_ref().map(|app| app.id) != Some(app_id) {
                return Ok(());
            }
            state.views.request += 1;
            state.views.loading = true;
            state.views.request
        };
        text("view-status", "保存した一覧を読み込み中…");
        self.update_view_controls();
        let response = api::<Vec<SavedView>>(&format!("/apps/{app_id}/views"), "GET", None).await;
        {
            let mut state = self.state.borrow_mut();
            if state.views.request != request
                || state.active.as_ref().map(|app| app.id) != Some(app_id)
            {
                return Ok(());
            }
            state.views.loading = false;
            match response {
                Ok(views) => {
                    state.views.list = views;
                    state.views.loaded = true;
                    text("view-status", "");
                }
                Err(_) => {
                    state.views.loaded = false;
                    text(
                        "view-status",
                        "保存した一覧を読み込めませんでした。「一覧を再読込」で再試行できます。レコードの操作は続けられます。",
                    );
                }
            }
        }
        self.render_views()?;
        Ok(())
    }

    pub async fn choose_view(&self) -> Result {
        let selected = {
            let state = self.state.borrow();
            if state.views.loading || !state.views.loaded || state.views.busy {
                return Ok(());
            }
            let raw = value(&id("record-view"));
            if raw.is_empty() {
                None
            } else {
                let Some(view) = state
                    .views
                    .list
                    .iter()
                    .find(|view| view.id.to_string() == raw)
                else {
                    return Ok(());
                };
                Some(view.clone())
            }
        };
        self.apply_view(selected).await
    }

    pub async fn apply_view(&self, selected: Option<SavedView>) -> Result {
        {
            let mut state = self.state.borrow_mut();
            if selected
                .as_ref()
                .is_some_and(|view| state.active.as_ref().map(|app| app.id) != Some(view.app_id))
            {
                return Ok(());
            }
            state.query = selected
                .as_ref()
                .map(|view| view.definition.query.clone())
                .unwrap_or_default();
            state.filter = selected
                .as_ref()
                .and_then(|view| view.definition.filter.clone());
            state.views.selected = selected;
        }
        self.restore_query_controls()?;
        self.reset_paging();
        self.render_views()?;
        self.load_records().await.map(|_| ())
    }

    pub fn restore_query_controls(&self) -> Result {
        let (query, filter) = {
            let state = self.state.borrow();
            (state.query.clone(), state.filter.clone())
        };
        set_value(&id("search"), &query);
        self.reset_filter_controls()?;
        if let Some(filter) = filter {
            crate::dom::query(&id("search-form"), ".filter-panel").set_attribute("open", "")?;
            set_value(&id("filter-field"), &filter.field);
            set_value(
                &id("filter-mode"),
                if filter.value.is_null() {
                    "empty"
                } else {
                    "equals"
                },
            );
            self.render_filter_value()?;
            if !filter.value.is_null() {
                let text = filter
                    .value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| filter.value.to_string());
                set_value(&id("filter-value"), &text);
            }
        }
        Ok(())
    }
}
