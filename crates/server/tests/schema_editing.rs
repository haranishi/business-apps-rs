// SPDX-License-Identifier: Apache-2.0
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::NewApp;
use business_apps_server::{AppError, router, store::Store};
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
        let count = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "business-apps-schema-{}-{stamp}-{count}.sqlite",
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
fn schema() -> NewApp {
    serde_json::from_value(json!({"name":"Tasks","fields":[{"key":"name","label":"Name","kind":"text","required":true},{"key":"status","label":"Status","kind":"select","options":["Open","Done"]}]})).unwrap()
}
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    data: Option<Value>,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "127.0.0.1:8080")
                .header("x-bizapp-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(data.map(|v| v.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 300000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn concurrent_schema_edits_keep_records_identical_and_survive_reopen() {
    let db = Database::new();
    let store = Store::open(&db.0).unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let other = store.create_app(schema()).await.unwrap();
    let values = json!({"name":"Existing","status":"Open"})
        .as_object()
        .unwrap()
        .clone();
    let record = store.create_record(app.id, values.clone()).await.unwrap();
    let record = store
        .update_record(app.id, record.id, 1, values.clone())
        .await
        .unwrap();
    let id = record.id;
    let before = store
        .call(move |c| {
            Ok(
                c.query_row("SELECT values_json FROM records WHERE id=?1", [id], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut updated = schema();
    updated.name = "New tasks".into();
    updated.fields[0].label = "Title".into();
    updated.fields.reverse();
    updated
        .fields
        .push(serde_json::from_value(json!({"key":"due","label":"Due","kind":"date"})).unwrap());
    let (one, two) = tokio::join!(
        store.update_app(app.id, 1, updated.clone()),
        store.update_app(app.id, 1, updated)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert!(matches!(one, Err(AppError::Conflict)) || matches!(two, Err(AppError::Conflict)));
    let saved = store.get_record(app.id, id).await.unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(saved.values, values);
    let after = store
        .call(move |c| {
            Ok(
                c.query_row("SELECT values_json FROM records WHERE id=?1", [id], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(store.get_app(other.id).await.unwrap().revision, 1);
    let loaded = store.get_app(app.id).await.unwrap();
    assert_eq!(loaded.revision, 2);
    assert_eq!(loaded.fields[0].key, "status");
    assert_eq!(loaded.fields[1].key, "name");
    assert_eq!(loaded.fields[1].label, "Title");
    drop(store);
    let reopened = Store::open(&db.0).unwrap();
    assert_eq!(reopened.get_app(app.id).await.unwrap().revision, 2);
    assert_eq!(
        reopened.get_record(app.id, id).await.unwrap().values,
        values
    );
}

#[tokio::test]
async fn api_schema_edits_reject_stale_destructive_and_invalid_revisions() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let api = router(store.clone());
    let path = format!("/api/v1/apps/{}", app.id);
    assert_eq!(request(&api, "GET", &path, None).await.1["revision"], 1);
    let mut body = serde_json::to_value(schema()).unwrap();
    body["revision"] = json!(1);
    body["name"] = json!("Updated");
    assert_eq!(
        request(&api, "PUT", &path, Some(body.clone())).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&api, "PUT", &path, Some(body.clone())).await.0,
        StatusCode::CONFLICT
    );
    body["revision"] = json!(0);
    assert_eq!(
        request(&api, "PUT", &path, Some(body.clone())).await.0,
        StatusCode::BAD_REQUEST
    );
    body["revision"] = json!(2);
    body["fields"] = json!([{"key":"new","label":"New","kind":"text"}]);
    assert_eq!(
        request(&api, "PUT", &path, Some(body)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(store.get_app(app.id).await.unwrap().revision, 2);
    assert_eq!(
        request(&api, "GET", "/api/v1/apps/99999", None).await.0,
        StatusCode::NOT_FOUND
    );
    let mut body = serde_json::to_value(schema()).unwrap();
    body["revision"] = json!(1);
    assert_eq!(
        request(&api, "PUT", "/api/v1/apps/99999", Some(body))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn format_two_migrates_choices_and_records_to_app_revision_one() {
    let db = Database::new();
    let connection = rusqlite::Connection::open(&db.0).unwrap();
    connection.execute_batch("CREATE TABLE apps(id INTEGER PRIMARY KEY, name TEXT NOT NULL, fields TEXT NOT NULL); CREATE TABLE records(id INTEGER PRIMARY KEY, app_id INTEGER NOT NULL REFERENCES apps(id), revision INTEGER NOT NULL, values_json TEXT NOT NULL); CREATE INDEX records_by_app_id ON records(app_id,id DESC); PRAGMA application_id=1111577171; PRAGMA user_version=2;").unwrap();
    let fields = serde_json::to_string(&schema().fields).unwrap();
    connection
        .execute("INSERT INTO apps VALUES(5,'Saved',?1)", [fields.clone()])
        .unwrap();
    connection
        .execute(
            "INSERT INTO records VALUES(9,5,4,?1)",
            [r#"{"name":"Old","status":"Done"}"#],
        )
        .unwrap();
    drop(connection);
    let store = Store::open(&db.0).unwrap();
    assert_eq!(store.get_app(5).await.unwrap().revision, 1);
    assert_eq!(
        store.get_app(5).await.unwrap().fields[1].options,
        ["Open", "Done"]
    );
    assert_eq!(store.get_record(5, 9).await.unwrap().revision, 4);
    store
        .call(move |c| {
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
                4
            );
            assert_eq!(
                c.query_row("SELECT fields FROM apps WHERE id=5", [], |r| r
                    .get::<_, String>(0))?,
                fields
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[test]
fn failed_owned_database_migration_leaves_metadata_and_rows_intact() {
    let db = Database::new();
    let connection = rusqlite::Connection::open(&db.0).unwrap();
    connection.execute_batch("CREATE TABLE apps(id INTEGER PRIMARY KEY, revision INTEGER, name TEXT); INSERT INTO apps VALUES(1,7,'keep'); PRAGMA application_id=1111577171; PRAGMA user_version=2;").unwrap();
    assert!(Store::open(&db.0).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row("SELECT name FROM apps WHERE id=1", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}
