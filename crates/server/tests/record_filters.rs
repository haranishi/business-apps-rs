// SPDX-License-Identifier: Apache-2.0
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::{NewApp, RecordFilter};
use business_apps_server::{AppError, router, store::Store};
use serde_json::{Value, json};
use tower::ServiceExt;

fn schema() -> NewApp {
    serde_json::from_value(json!({"name":"Filters", "fields":[
        {"key":"name","label":"Name","kind":"text","required":true},
        {"key":"note","label":"Note","kind":"textarea"},
        {"key":"amount","label":"Amount","kind":"number"},
        {"key":"done","label":"Done","kind":"boolean"},
        {"key":"due","label":"Due","kind":"date"},
        {"key":"status","label":"Status","kind":"select","options":["未対応","完了"]}
    ]}))
    .unwrap()
}
fn filter(field: &str, value: Value) -> Option<RecordFilter> {
    Some(RecordFilter {
        field: field.into(),
        value,
    })
}
fn encode(raw: &str) -> String {
    raw.bytes().map(|byte| format!("%{byte:02X}")).collect()
}

async fn ids(store: &Store, app_id: i64, field: &str, value: Value) -> Vec<i64> {
    store
        .list_records_filtered(app_id, None, 100, None, filter(field, value))
        .await
        .unwrap()
        .records
        .iter()
        .map(|record| record.id)
        .collect()
}

#[tokio::test]
async fn typed_filters_compose_with_search_and_select_only_the_named_field() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let mut saved = Vec::new();
    for values in [
        json!({"name":"東京", "note":"literal %_'", "amount":0,"done":false,"due":"2024-02-29","status":"未対応"}),
        json!({"name":"東京", "note":"", "amount":1.5,"done":true,"due":null,"status":"完了"}),
        json!({"name":"other", "note":null, "amount":0,"done":false,"status":"未対応"}),
        json!({"name":"東京' OR 1=1 --", "note":" ", "amount":-2,"done":false,"due":"2026-10-15","status":"完了"}),
    ] {
        saved.push(
            store
                .create_record(app.id, values.as_object().unwrap().clone())
                .await
                .unwrap()
                .id,
        );
    }
    assert_eq!(
        ids(&store, app.id, "name", json!("東京")).await,
        [saved[1], saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "note", json!("literal %_'")).await,
        [saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "amount", json!(0.0)).await,
        [saved[2], saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "done", json!(false)).await,
        [saved[3], saved[2], saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "due", json!("2024-02-29")).await,
        [saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "status", json!("完了")).await,
        [saved[3], saved[1]]
    );
    assert_eq!(
        ids(&store, app.id, "name", json!("東京' OR 1=1 --")).await,
        [saved[3]]
    );
    assert!(ids(&store, app.id, "name", json!("TOKYO")).await.is_empty());
    let page = store
        .list_records_filtered(
            app.id,
            None,
            100,
            Some("東京".into()),
            filter("done", json!(false)),
        )
        .await
        .unwrap();
    assert_eq!(
        page.records
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        [saved[3], saved[0]]
    );
    let other = store.create_app(schema()).await.unwrap();
    assert!(ids(&store, other.id, "done", json!(false)).await.is_empty());
}

#[tokio::test]
async fn empty_filters_include_missing_null_and_empty_text_but_preserve_whitespace_zero_false() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let mut saved = Vec::new();
    for values in [
        json!({"name":"missing"}),
        json!({"name":"null","note":null}),
        json!({"name":"empty","note":"","amount":0,"done":false}),
        json!({"name":"space","note":" "}),
    ] {
        saved.push(
            store
                .create_record(app.id, values.as_object().unwrap().clone())
                .await
                .unwrap()
                .id,
        );
    }
    assert_eq!(
        ids(&store, app.id, "note", Value::Null).await,
        [saved[2], saved[1], saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "amount", Value::Null).await,
        [saved[3], saved[1], saved[0]]
    );
    assert_eq!(
        ids(&store, app.id, "done", Value::Null).await,
        [saved[3], saved[1], saved[0]]
    );
    assert!(ids(&store, app.id, "name", Value::Null).await.is_empty());
    assert_eq!(ids(&store, app.id, "note", json!("")).await, [saved[2]]);
}

#[tokio::test]
async fn filtered_cursors_and_csv_cover_the_same_complete_result() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    for index in 0..37 {
        store
            .create_record(
                app.id,
                json!({"name":format!("row {index:02}"),"done":index >= 32})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .await
            .unwrap();
    }
    let selected = filter("done", json!(false));
    let first = store
        .list_records_filtered(app.id, None, 30, None, selected.clone())
        .await
        .unwrap();
    let second = store
        .list_records_filtered(app.id, first.next_cursor, 30, None, selected.clone())
        .await
        .unwrap();
    assert_eq!(first.records.len(), 30);
    assert_eq!(second.records.len(), 2);
    assert!(second.next_cursor.is_none());
    let csv = store
        .export_csv_filtered(app.id, None, selected)
        .await
        .unwrap();
    let expected: Vec<_> = first
        .records
        .iter()
        .chain(&second.records)
        .map(|record| format!("\"{}\",", record.id))
        .collect();
    assert_eq!(
        csv.lines()
            .skip(1)
            .map(|line| line.split_once(',').unwrap().0.to_owned() + ",")
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        store
            .list_records(app.id, None, 100, None)
            .await
            .unwrap()
            .records
            .len(),
        37
    );
}

#[tokio::test]
async fn http_filter_parsing_rejects_wrong_shapes_and_values_and_exports_same_selection() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema()).await.unwrap();
    store
        .create_record(
            app.id,
            json!({"name":"selected","done":false})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    for raw in [
        "not json",
        "{}",
        r#"{"field":"done","value":0}"#,
        r#"{"field":"other","value":null}"#,
        r#"{"field":"name","value":"x","extra":1}"#,
    ] {
        for suffix in ["records", "records.csv"] {
            let response = router(store.clone())
                .oneshot(
                    Request::builder()
                        .uri(format!(
                            "/api/v1/apps/{}/{suffix}?filter={}",
                            app.id,
                            encode(raw)
                        ))
                        .header("host", "localhost")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{raw}");
        }
    }
    let encoded = encode(r#"{"field":"done","value":false}"#);
    let response = router(store.clone())
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/apps/{}/records.csv?filter={encoded}",
                    app.id
                ))
                .header("host", "localhost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let csv =
        String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap();
    assert!(csv.contains("selected"));
    assert!(matches!(
        store
            .list_records_filtered(app.id, None, 50, None, filter("done", json!(0)))
            .await,
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        store
            .export_csv_filtered(999999, None, filter("done", json!(false)))
            .await,
        Err(AppError::NotFound)
    ));
}

#[tokio::test]
async fn filters_and_csv_preserve_saved_values_after_database_reopen() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "business-apps-filter-{}-{unique}.sqlite",
        std::process::id()
    ));
    let store = Store::open(&path).unwrap();
    let app = store.create_app(schema()).await.unwrap();
    let record = store
        .create_record(
            app.id,
            json!({"name":"東京", "note":"before\r\nafter", "amount":0,"done":false})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    let before = store
        .export_csv_filtered(app.id, None, filter("done", json!(false)))
        .await
        .unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened.get_record(app.id, record.id).await.unwrap().values,
        record.values
    );
    assert_eq!(
        reopened
            .export_csv_filtered(app.id, None, filter("done", json!(false)))
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        ids(&reopened, app.id, "amount", json!(0)).await,
        [record.id]
    );
    reopened
        .call(|connection| {
            connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
            Ok(())
        })
        .await
        .unwrap();
    drop(reopened);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
}
