// SPDX-License-Identifier: Apache-2.0
pub mod store;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderValue, Method, StatusCode, Uri, uri::Authority},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use business_apps_domain::{App, NewApp, Record, ValidationError, Values};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use store::Store;

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    NotFound,
    Conflict,
    Unavailable,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AppError {}
impl From<ValidationError> for AppError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error.0)
    }
}
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}
impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        Self::Unavailable
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Validation(message) => (StatusCode::BAD_REQUEST, "validation", message),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "App or record not found.".into(),
            ),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "conflict",
                "This app or record changed. Reload it before saving again.".into(),
            ),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Storage is unavailable.".into(),
            ),
        };
        (
            status,
            Json(json!({"error":{"code":code,"message":message}})),
        )
            .into_response()
    }
}

pub fn local_address(input: &str) -> Result<SocketAddr, std::io::Error> {
    let address: SocketAddr = input
        .parse()
        .map_err(|_| std::io::Error::other("BIZAPP_BIND must be a loopback socket address."))?;
    if !address.ip().is_loopback() {
        return Err(std::io::Error::other(
            "Network listeners require an authentication design; use loopback.",
        ));
    }
    Ok(address)
}

pub fn router(store: Store) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../../../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css; charset=utf-8")],
                    include_str!("../../../web/style.css"),
                )
            }),
        )
        .route("/favicon.ico", get(|| async { StatusCode::NO_CONTENT }))
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/api/v1/apps", get(list_apps).post(create_app))
        .route("/api/v1/apps/{app_id}", get(get_app).put(update_app))
        .route(
            "/api/v1/apps/{app_id}/records",
            get(list_records).post(create_record),
        )
        .route(
            "/api/v1/apps/{app_id}/records/{id}",
            get(get_record).put(update_record).delete(delete_record),
        )
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(middleware::from_fn(local_guard))
        .with_state(store)
}

async fn local_guard(request: Request, next: Next) -> Response {
    let denied = || {
        (
            StatusCode::FORBIDDEN,
            "Only local, same-origin requests are supported.",
        )
            .into_response()
    };
    let Some(host) = request
        .headers()
        .get("host")
        .and_then(|value| value.to_str().ok())
    else {
        return denied();
    };
    let Ok(authority) = host.parse::<Authority>() else {
        return denied();
    };
    if host.contains('@')
        || !matches!(
            authority.host(),
            "localhost" | "127.0.0.1" | "[::1]" | "::1"
        )
    {
        return denied();
    }
    if let Some(origin) = request.headers().get("origin") {
        let Ok(origin) = origin.to_str().unwrap_or("").parse::<Uri>() else {
            return denied();
        };
        if origin.scheme_str() != Some("http")
            || origin.authority().map(Authority::as_str) != Some(host)
        {
            return denied();
        }
    }
    if request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|value| value == "cross-site")
    {
        return denied();
    }
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) && request
        .headers()
        .get("x-bizapp-request")
        .is_none_or(|value| value != "1")
    {
        return denied();
    }
    let mut response = next.run(request).await;
    for (key, value) in [
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
    ] {
        response
            .headers_mut()
            .insert(key, HeaderValue::from_static(value));
    }
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateApp {
    revision: i64,
    name: String,
    fields: Vec<business_apps_domain::Field>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewRecord {
    values: Values,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateRecord {
    revision: i64,
    values: Values,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    limit: Option<usize>,
    before: Option<i64>,
    q: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteQuery {
    revision: i64,
}

async fn list_apps(
    State(store): State<Store>,
) -> Result<Json<Vec<business_apps_domain::App>>, AppError> {
    Ok(Json(store.list_apps().await?))
}
async fn create_app(
    State(store): State<Store>,
    Json(input): Json<NewApp>,
) -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::CREATED, Json(store.create_app(input).await?)))
}
async fn get_app(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
) -> Result<Json<App>, AppError> {
    Ok(Json(store.get_app(app_id).await?))
}
async fn update_app(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
    Json(input): Json<UpdateApp>,
) -> Result<Json<App>, AppError> {
    Ok(Json(
        store
            .update_app(
                app_id,
                input.revision,
                NewApp {
                    name: input.name,
                    fields: input.fields,
                },
            )
            .await?,
    ))
}
async fn list_records(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
    Query(query): Query<ListQuery>,
) -> Result<Json<store::RecordPage>, AppError> {
    Ok(Json(
        store
            .list_records(app_id, query.before, query.limit.unwrap_or(50), query.q)
            .await?,
    ))
}
async fn create_record(
    State(store): State<Store>,
    Path(app_id): Path<i64>,
    Json(input): Json<NewRecord>,
) -> Result<impl IntoResponse, AppError> {
    Ok((
        StatusCode::CREATED,
        Json(store.create_record(app_id, input.values).await?),
    ))
}
async fn get_record(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
) -> Result<Json<Record>, AppError> {
    Ok(Json(store.get_record(app_id, id).await?))
}
async fn update_record(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
    Json(input): Json<UpdateRecord>,
) -> Result<Json<Record>, AppError> {
    Ok(Json(
        store
            .update_record(app_id, id, input.revision, input.values)
            .await?,
    ))
}
async fn delete_record(
    State(store): State<Store>,
    Path((app_id, id)): Path<(i64, i64)>,
    Query(input): Query<DeleteQuery>,
) -> Result<StatusCode, AppError> {
    store.delete_record(app_id, id, input.revision).await?;
    Ok(StatusCode::NO_CONTENT)
}
