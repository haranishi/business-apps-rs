// SPDX-License-Identifier: Apache-2.0
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::NewApp;
use business_apps_server::{router, store::Store};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "business-apps-rich-{}-{stamp}-{sequence}.sqlite",
            std::process::id()
        );
        Self(std::env::temp_dir().join(name))
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

#[tokio::test]
async fn version_one_upgrades_without_rewriting_saved_values_or_revisions() {
    let db = Database::new();
    let connection = rusqlite::Connection::open(&db.0).unwrap();
    connection.execute_batch("CREATE TABLE apps (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, fields TEXT NOT NULL CHECK(json_valid(fields)));
        CREATE TABLE records (id INTEGER PRIMARY KEY AUTOINCREMENT, app_id INTEGER NOT NULL REFERENCES apps(id), revision INTEGER NOT NULL CHECK(revision > 0), values_json TEXT NOT NULL CHECK(json_valid(values_json)));
        CREATE INDEX records_by_app_id ON records(app_id, id DESC);
        PRAGMA application_id = 1111577171; PRAGMA user_version = 1;").unwrap();
    let fields = r#"[{"key":"name","label":"Name","kind":"text","required":true}]"#;
    let saved = r#"{"name":"既存データ"}"#;
    connection
        .execute("INSERT INTO apps VALUES (7, 'Legacy', ?1)", [fields])
        .unwrap();
    connection
        .execute("INSERT INTO records VALUES (12, 7, 4, ?1)", [saved])
        .unwrap();
    drop(connection);
    let store = Store::open(&db.0).unwrap();
    let app = &store.list_apps().await.unwrap()[0];
    assert_eq!(app.id, 7);
    assert!(app.fields[0].options.is_empty());
    assert_eq!(store.get_record(7, 12).await.unwrap().revision, 4);
    store
        .call(move |c| {
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
                4
            );
            assert_eq!(
                c.query_row("SELECT fields FROM apps WHERE id=7", [], |r| r
                    .get::<_, String>(0))?,
                fields
            );
            assert_eq!(
                c.query_row("SELECT values_json FROM records WHERE id=12", [], |r| {
                    r.get::<_, String>(0)
                })?,
                saved
            );
            Ok(())
        })
        .await
        .unwrap();
    drop(store);
    assert_eq!(
        Store::open(&db.0)
            .unwrap()
            .get_record(7, 12)
            .await
            .unwrap()
            .values["name"],
        "既存データ"
    );
}

#[tokio::test]
async fn rich_values_reject_invalid_api_writes_and_survive_reopening() {
    let db = Database::new();
    let store = Store::open(&db.0).unwrap();
    let input: NewApp = serde_json::from_value(json!({"name":"Tasks","fields":[
        {"key":"due","label":"Due","kind":"date","required":true},
        {"key":"status","label":"Status","kind":"select","required":true,"options":["未対応","完了"]},
        {"key":"note","label":"Note","kind":"textarea"}
    ]})).unwrap();
    let app = store.create_app(input).await.unwrap();
    let api = router(store.clone());
    for values in [
        json!({"due":"2025-02-29","status":"未対応"}),
        json!({"due":"2024-02-29","status":"unknown"}),
    ] {
        let response = api
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/apps/{}/records", app.id))
                    .header("host", "127.0.0.1:8080")
                    .header("x-bizapp-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"values":values}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 10000).await.unwrap()).unwrap();
        assert_eq!(error["error"]["code"], "validation");
    }
    assert!(
        store
            .list_records(app.id, None, 50, None)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    let values =
        json!({"due":"2024-02-29","status":"未対応","note":"one\n二行目\n<script>plain</script>"});
    let record = store
        .create_record(app.id, values.as_object().unwrap().clone())
        .await
        .unwrap();
    drop(api);
    drop(store);
    let reopened = Store::open(&db.0).unwrap();
    assert_eq!(
        reopened.list_apps().await.unwrap()[0].fields[1].options,
        ["未対応", "完了"]
    );
    assert_eq!(
        reopened.get_record(app.id, record.id).await.unwrap().values,
        *values.as_object().unwrap()
    );
}
