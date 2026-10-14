// SPDX-License-Identifier: Apache-2.0
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::{NewApp, Values};
use business_apps_server::{AppError, local_address, router, store::Store};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "127.0.0.1:8080")
        .header("x-bizapp-request", "1");
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 512 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn schema(name: &str) -> NewApp {
    serde_json::from_value(
        json!({"name":name,"fields":[{"key":"name","label":"Name","kind":"text","required":true}]}),
    )
    .unwrap()
}
fn values(name: &str) -> Values {
    json!({"name":name}).as_object().unwrap().clone()
}

#[tokio::test]
async fn crud_is_scoped_and_invalid_input_never_persists() {
    let store = Store::open(":memory:").unwrap();
    let first = store.create_app(schema("First")).await.unwrap();
    let second = store.create_app(schema("Second")).await.unwrap();
    let app = router(store.clone());
    let (status, record) = request(
        &app,
        "POST",
        &format!("/api/v1/apps/{}/records", first.id),
        Some(json!({"values":{"name":"東京 <script>example</script>"}})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = record["id"].as_i64().unwrap();
    let wrong_path = format!("/api/v1/apps/{}/records/{id}", second.id);
    for (method, path, body) in [
        ("GET", wrong_path.clone(), None),
        (
            "PUT",
            wrong_path.clone(),
            Some(json!({"revision":1,"values":{"name":"intrusion"}})),
        ),
        ("DELETE", format!("{wrong_path}?revision=1"), None),
    ] {
        assert_eq!(
            request(&app, method, &path, body).await.0,
            StatusCode::NOT_FOUND
        );
    }
    for invalid in [
        json!({"name":" "}),
        json!({"name":12}),
        json!({"name":"x","unknown":true}),
    ] {
        assert_eq!(
            request(
                &app,
                "POST",
                &format!("/api/v1/apps/{}/records", first.id),
                Some(json!({"values":invalid}))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let records = store.list_records(first.id, None, 50, None).await.unwrap();
    assert_eq!(records.records.len(), 1);
    assert_eq!(
        records.records[0].values["name"],
        "東京 <script>example</script>"
    );
    assert!(
        store
            .list_records(second.id, None, 50, None)
            .await
            .unwrap()
            .records
            .is_empty()
    );
}

#[tokio::test]
async fn revisions_protect_updates_and_deletes() {
    let store = Store::open(":memory:").unwrap();
    let app_id = store.create_app(schema("Versions")).await.unwrap().id;
    let record = store
        .create_record(app_id, values("original"))
        .await
        .unwrap();
    let app = router(store.clone());
    let path = format!("/api/v1/apps/{app_id}/records/{}", record.id);
    let (status, updated) = request(
        &app,
        "PUT",
        &path,
        Some(json!({"revision":1,"values":{"name":"new"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["revision"], 2);
    let (status, error) = request(
        &app,
        "PUT",
        &path,
        Some(json!({"revision":1,"values":{"name":"stale"}})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "conflict");
    assert_eq!(
        request(&app, "DELETE", &format!("{path}?revision=1"), None)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store.get_record(app_id, record.id).await.unwrap().values["name"],
        "new"
    );
    assert_eq!(
        request(&app, "DELETE", &format!("{path}?revision=2"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", &path, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn two_concurrent_editors_have_exactly_one_winner() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema("Concurrent")).await.unwrap();
    let record = store
        .create_record(app.id, values("original"))
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        store.update_record(app.id, record.id, 1, values("one")),
        store.update_record(app.id, record.id, 1, values("two"))
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert!(matches!(one, Err(AppError::Conflict)) || matches!(two, Err(AppError::Conflict)));
    assert_eq!(
        store.get_record(app.id, record.id).await.unwrap().revision,
        2
    );
}

#[tokio::test]
async fn cursor_pages_are_complete_and_search_is_literal() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema("Pages")).await.unwrap();
    for i in 0..75 {
        store
            .create_record(app.id, values(&format!("Customer {i}")))
            .await
            .unwrap();
    }
    store
        .create_record(app.id, values("東京 a%_ ' OR 1=1 --"))
        .await
        .unwrap();
    let mut ids = Vec::new();
    let mut before = None;
    loop {
        let page = store.list_records(app.id, before, 30, None).await.unwrap();
        ids.extend(page.records.iter().map(|r| r.id));
        before = page.next_cursor;
        if before.is_none() {
            break;
        }
    }
    assert_eq!(ids.len(), 76);
    assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), 76);
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    for query in ["東京", "a%_", "' OR 1=1 --"] {
        assert_eq!(
            store
                .list_records(app.id, None, 100, Some(query.into()))
                .await
                .unwrap()
                .records
                .len(),
            1
        );
    }
    assert_eq!(
        store
            .list_records(app.id, None, 100, Some("customer".into()))
            .await
            .unwrap()
            .records
            .len(),
        75
    );
    assert!(
        store
            .list_records(app.id, None, 100, Some("name".into()))
            .await
            .unwrap()
            .records
            .is_empty()
    );
    let router = router(store);
    for query in [
        "limit=0",
        "limit=101",
        "before=0",
        "q=", /* empty search is accepted */
    ] {
        let status = request(
            &router,
            "GET",
            &format!("/api/v1/apps/{}/records?{query}", app.id),
            None,
        )
        .await
        .0;
        assert_eq!(
            status,
            if query == "q=" {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            }
        );
    }
    assert_eq!(
        request(&router, "GET", "/api/v1/apps/99999/records", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

struct TestDatabase(PathBuf);
impl TestDatabase {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("business-apps-{}-{stamp}.sqlite", std::process::id());
        Self(std::env::temp_dir().join(name))
    }
}
impl Drop for TestDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

#[tokio::test]
async fn reopening_preserves_schema_records_and_revisions() {
    let database = TestDatabase::new();
    let store = Store::open(&database.0).unwrap();
    let app = store.create_app(schema("Saved")).await.unwrap();
    let record = store
        .create_record(app.id, values("durable"))
        .await
        .unwrap();
    store
        .update_record(app.id, record.id, 1, values("persisted"))
        .await
        .unwrap();
    drop(store);
    let reopened = Store::open(&database.0).unwrap();
    assert_eq!(reopened.list_apps().await.unwrap()[0].fields[0].key, "name");
    let loaded = reopened.get_record(app.id, record.id).await.unwrap();
    assert_eq!(loaded.revision, 2);
    assert_eq!(loaded.values["name"], "persisted");
    assert!(
        reopened
            .call(|c| Ok(c.pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))?))
            .await
            .unwrap()
    );
}

#[test]
fn unrelated_and_future_databases_are_refused_without_migration() {
    let unrelated = TestDatabase::new();
    let connection = rusqlite::Connection::open(&unrelated.0).unwrap();
    connection
        .execute_batch("CREATE TABLE unrelated(value TEXT);")
        .unwrap();
    assert!(Store::open(&unrelated.0).is_err());
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'apps'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let future = TestDatabase::new();
    drop(Store::open(&future.0).unwrap());
    let connection = rusqlite::Connection::open(&future.0).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    assert!(Store::open(&future.0).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

#[tokio::test]
async fn local_origin_host_and_mutation_boundaries_are_enforced() {
    let app = router(Store::open(":memory:").unwrap());
    for (host, origin, method, marker, expected) in [
        ("evil.example", None, "GET", false, StatusCode::FORBIDDEN),
        (
            "127.0.0.1:8080",
            Some("https://evil.example"),
            "GET",
            false,
            StatusCode::FORBIDDEN,
        ),
        (
            "127.0.0.1:8080",
            Some("null"),
            "GET",
            false,
            StatusCode::FORBIDDEN,
        ),
        ("127.0.0.1:8080", None, "POST", false, StatusCode::FORBIDDEN),
        (
            "127.0.0.1:8080",
            Some("http://127.0.0.1:8080"),
            "GET",
            false,
            StatusCode::OK,
        ),
        ("[::1]:8080", None, "GET", false, StatusCode::OK),
        ("localhost:8080", None, "GET", false, StatusCode::OK),
    ] {
        let mut builder = Request::builder()
            .uri("/health")
            .method(method)
            .header("host", host);
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        if marker {
            builder = builder.header("x-bizapp-request", "1");
        }
        let response = app
            .clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{host} {origin:?} {method}");
        if expected == StatusCode::OK {
            assert!(response.headers().contains_key("content-security-policy"));
        }
    }
    assert!(local_address("0.0.0.0:8080").is_err());
    assert!(local_address("[::]:8080").is_err());
    assert!(local_address("127.0.0.1:8080").is_ok());
    assert!(local_address("[::1]:8080").is_ok());
}

#[tokio::test]
async fn oversized_bodies_are_rejected_before_creating_an_app() {
    let store = Store::open(":memory:").unwrap();
    let app = router(store.clone());
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/apps",
            Some(json!({"name":"x".repeat(270_000),"fields":[]}))
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(store.list_apps().await.unwrap().is_empty());
}
