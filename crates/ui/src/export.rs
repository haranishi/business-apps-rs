// SPDX-License-Identifier: Apache-2.0
use crate::{
    dom::*,
    runtime::{ApiError, Ui, api_response},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, HtmlAnchorElement, Url};

impl Ui {
    pub async fn export_csv(&self) -> Result {
        let (request, app_id, query) = {
            let mut state = self.state.borrow_mut();
            if state.loading || !state.export_ready || state.exporting {
                return Ok(());
            }
            let Some(app) = &state.active else {
                return Ok(());
            };
            let context = (
                state.request,
                app.id,
                crate::filters::query_parameters(&state.query, state.filter.as_ref()),
            );
            state.exporting = true;
            context
        };
        disabled(&id("export-csv"), true);
        text("export-status", "CSVを準備中…");
        let path = format!("/apps/{app_id}/records.csv?{query}");
        let result = async {
            let response = api_response(&path, "GET", None).await?;
            let blob = JsFuture::from(response.blob()?)
                .await?
                .dyn_into::<Blob>()
                .map_err(ApiError::from)?;
            Ok::<_, ApiError>(blob)
        }
        .await;
        {
            let mut state = self.state.borrow_mut();
            state.exporting = false;
            // Keep the table's existing buttons alive: an open record dialog
            // may need its invoking button when native focus is restored.
            disabled(&id("export-csv"), state.loading || !state.export_ready);
        }
        // A page/search/app reload supersedes an export, including its feedback.
        if request != self.state.borrow().request {
            return Ok(());
        }
        match result {
            Ok(blob) => {
                let url = Url::create_object_url_with_blob(&blob)?;
                let anchor = element("a", None, None)?.unchecked_into::<HtmlAnchorElement>();
                anchor.set_href(&url);
                anchor.set_download(&format!("app-{app_id}-records.csv"));
                anchor.click();
                Url::revoke_object_url(&url)?;
                text(
                    "export-status",
                    "CSVを出力しました。ダウンロード先を確認してください。",
                );
            }
            Err(error) => text("export-status", &error.message),
        }
        Ok(())
    }
}
