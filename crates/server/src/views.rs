// SPDX-License-Identifier: Apache-2.0
use crate::{AppError, DeleteQuery, store::Store};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use business_apps_domain::{SavedView, ViewDefinition};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateView {
    revision: i64,
    definition: ViewDefinition,
}

pub(super) async fn list(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
) -> Result<Json<Vec<SavedView>>, AppError> {
    Ok(Json(store.list_views(app_id).await?))
}
pub(super) async fn create(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
    Json(definition): Json<ViewDefinition>,
) -> Result<(StatusCode, Json<SavedView>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(store.create_view(app_id, definition).await?),
    ))
}
pub(super) async fn read(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
) -> Result<Json<SavedView>, AppError> {
    Ok(Json(store.get_view(app_id, id).await?))
}
pub(super) async fn update(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
    Json(input): Json<UpdateView>,
) -> Result<Json<SavedView>, AppError> {
    Ok(Json(
        store
            .update_view(app_id, id, input.revision, input.definition)
            .await?,
    ))
}
pub(super) async fn delete(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
    Query(input): Query<DeleteQuery>,
) -> Result<StatusCode, AppError> {
    store.delete_view(app_id, id, input.revision).await?;
    Ok(StatusCode::NO_CONTENT)
}
