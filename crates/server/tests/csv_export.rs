// SPDX-License-Identifier: Apache-2.0
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::NewApp;
use business_apps_server::{AppError, router, store::Store};
use serde_json::{Value, json};
use tower::ServiceExt;

fn schema() -> NewApp {
    serde_json::from_value(json!({"name":"Export", "fields":[
        {"key":"name","label":"=Name","kind":"text"},
        {"key":"note","label":"メモ","kind":"textarea"},
        {"key":"amount","label":"数量","kind":"number"},
        {"key":"active","label":"有効","kind":"boolean"},
        {"key":"due","label":"期限","kind":"date"},
        {"key":"status","label":"状況","kind":"select","options":["未対応","完了"]}
    ]}))
    .unwrap()
}

async fn export(store: &Store, path: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = router(store.clone())
        .oneshot(
            Request::builder()
                .uri(path)
                .header("host", "127.0.0.1:8080")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 17 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    (status, headers, bytes)
}

#[tokio::test]
async fn exports_schema_order_types_escaped_text_and_empty_added_fields() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let record = store.create_record(app.id, json!({"name":" \t=1+1", "note":"東京,\"会社\"\r\n二行目", "amount":-2.5,"active":false,"due":"2024-02-29","status":"完了"}).as_object().unwrap().clone()).await.unwrap();
    let csv = store.export_csv(app.id, None).await.unwrap();
    assert_eq!(
        csv,
        format!(
            "\u{feff}\"ID\",\"Revision\",\"'=Name\",\"メモ\",\"数量\",\"有効\",\"期限\",\"状況\"\r\n\"{}\",\"1\",\"' \t=1+1\",\"東京,\"\"会社\"\"\r\n二行目\",\"-2.5\",\"false\",\"2024-02-29\",\"完了\"\r\n",
            record.id
        )
    );
    let mut replacement = schema();
    replacement.fields.reverse();
    store.update_app(app.id, 1, replacement).await.unwrap();
    let csv = store.export_csv(app.id, None).await.unwrap();
    assert!(csv.starts_with(
        "\u{feff}\"ID\",\"Revision\",\"状況\",\"期限\",\"有効\",\"数量\",\"メモ\",\"'=Name\"\r\n"
    ));
    store
        .create_record(
            app.id,
            json!({"name":"zero", "amount":0,"active":false})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    assert!(
        store
            .export_csv(app.id, Some("zero".into()))
            .await
            .unwrap()
            .contains("\"\",\"\",\"false\",\"0\",\"\",\"zero\"\r\n")
    );
}

#[tokio::test]
async fn complete_export_is_app_scoped_literal_search_and_not_a_page() {
    let store = Store::open(":memory:").unwrap();
    let first = store.create_app(schema()).await.unwrap();
    let second = store.create_app(schema()).await.unwrap();
    for index in 0..35 {
        store
            .create_record(
                first.id,
                json!({"name":format!("東京%_{index:02}")})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .await
            .unwrap();
    }
    store
        .create_record(
            second.id,
            json!({"name":"other app"}).as_object().unwrap().clone(),
        )
        .await
        .unwrap();
    let csv = store.export_csv(first.id, None).await.unwrap();
    assert_eq!(csv.lines().count(), 36);
    assert!(csv.lines().nth(1).unwrap().contains("東京%_34"));
    assert!(csv.lines().last().unwrap().contains("東京%_00"));
    assert!(!csv.contains("other app"));
    assert_eq!(
        store.export_csv(first.id, Some("%_".into())).await.unwrap(),
        csv
    );
    assert_eq!(
        store
            .export_csv(first.id, Some("nonexistent".into()))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(matches!(
        store.export_csv(first.id, Some("あ".repeat(129))).await,
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        store.export_csv(i64::MAX, None).await,
        Err(AppError::NotFound)
    ));
    let (status, headers, body) = export(
        &store,
        &format!("/api/v1/apps/{}/records.csv?q=%25_", first.id),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "text/csv; charset=utf-8");
    assert_eq!(
        headers["content-disposition"],
        format!("attachment; filename=\"app-{}-records.csv\"", first.id)
    );
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(body, csv.as_bytes());
    assert_eq!(
        export(
            &store,
            &format!("/api/v1/apps/{}/records.csv?before=1", first.id)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        export(&store, "/api/v1/apps/999999/records.csv").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        store
            .export_csv(second.id, Some("東京".into()))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
}

async fn insert_many(store: &Store, app_id: i64, count: usize, name: String) {
    store
        .call(move |connection| {
            let transaction = connection.transaction()?;
            let values = json!({"name":name}).to_string();
            for _ in 0..count {
                transaction.execute(
                    "INSERT INTO records(app_id, revision, values_json) VALUES (?1, 1, ?2)",
                    rusqlite::params![app_id, values],
                )?;
            }
            transaction.commit()?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn row_limit_rejects_complete_response_and_search_recovers() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    insert_many(&store, app.id, 10_000, "many".into()).await;
    assert_eq!(
        store
            .export_csv(app.id, None)
            .await
            .unwrap()
            .lines()
            .count(),
        10_001
    );
    insert_many(&store, app.id, 1, "unique".into()).await;
    let (status, headers, bytes) =
        export(&store, &format!("/api/v1/apps/{}/records.csv", app.id)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(headers.get("content-disposition").is_none());
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["error"]["code"],
        "export_limit"
    );
    assert_eq!(
        store
            .export_csv(app.id, Some("unique".into()))
            .await
            .unwrap()
            .lines()
            .count(),
        2
    );
    assert_eq!(
        store
            .list_records(app.id, None, 100, None)
            .await
            .unwrap()
            .records
            .len(),
        100
    );
}

#[tokio::test]
async fn encoded_byte_limit_rejects_without_partial_download() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    insert_many(&store, app.id, 2_100, "\"".repeat(4_000)).await;
    let (status, headers, bytes) =
        export(&store, &format!("/api/v1/apps/{}/records.csv", app.id)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(headers.get("content-disposition").is_none());
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["error"]["code"],
        "export_limit"
    );
}

#[tokio::test]
async fn csv_get_obeys_the_existing_browser_boundary() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    for (header, value) in [
        ("host", "evil.example"),
        ("origin", "http://evil.example"),
        ("sec-fetch-site", "cross-site"),
    ] {
        let mut builder = Request::builder().uri(format!("/api/v1/apps/{}/records.csv", app.id));
        if header != "host" {
            builder = builder.header("host", "127.0.0.1:8080");
        }
        let response = router(store.clone())
            .oneshot(builder.header(header, value).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(response.headers().get("content-disposition").is_none());
    }
}
