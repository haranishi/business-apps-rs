// SPDX-License-Identifier: Apache-2.0
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_domain::{NewApp, ViewDefinition};
use business_apps_server::{AppError, router, store::Store};
use serde_json::{Value, json};
use tower::ServiceExt;

fn schema(name: &str) -> NewApp {
    serde_json::from_value(json!({"name":name,"fields":[{"key":"name","label":"Name","kind":"text","required":true},{"key":"done","label":"Done","kind":"boolean"}]})).unwrap()
}
fn definition(name: &str) -> ViewDefinition {
    serde_json::from_value(json!({"name":name,"query":"東京","filter":{"field":"done","value":false},"columns":["done","name"]})).unwrap()
}
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    marker: Option<&str>,
    extra: Option<(&str, &str)>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost:8080")
        .header("content-type", "application/json");
    if let Some(marker) = marker {
        builder = builder.header("x-bizapp-request", marker);
    }
    if let Some((key, value)) = extra {
        builder = builder.header(key, value);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 256 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn view_crud_compares_revisions_scopes_apps_and_never_changes_records() {
    let store = Store::open(":memory:").unwrap();
    let first = store.create_app(schema("First")).await.unwrap();
    let second = store.create_app(schema("Second")).await.unwrap();
    let record = store
        .create_record(
            first.id,
            json!({"name":"東京", "done":false})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    let view = store
        .create_view(first.id, definition("待ち"))
        .await
        .unwrap();
    assert_eq!(view.revision, 1);
    assert_eq!(view.definition.columns, ["done", "name"]);
    assert!(store.list_views(second.id).await.unwrap().is_empty());
    assert!(matches!(
        store.get_view(second.id, view.id).await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        store
            .update_view(second.id, view.id, 1, definition("Wrong"))
            .await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        store.delete_view(second.id, view.id, 1).await,
        Err(AppError::NotFound)
    ));
    let changed = store
        .update_view(first.id, view.id, 1, definition("更新"))
        .await
        .unwrap();
    assert_eq!(changed.revision, 2);
    assert!(matches!(
        store
            .update_view(first.id, view.id, 1, definition("Stale"))
            .await,
        Err(AppError::Conflict)
    ));
    assert!(matches!(
        store.delete_view(first.id, view.id, 1).await,
        Err(AppError::Conflict)
    ));
    for revision in [0, -1] {
        assert!(matches!(
            store
                .update_view(first.id, view.id, revision, definition("Invalid"))
                .await,
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            store.delete_view(first.id, view.id, revision).await,
            Err(AppError::Validation(_))
        ));
    }
    store.delete_view(first.id, view.id, 2).await.unwrap();
    assert!(store.list_views(first.id).await.unwrap().is_empty());
    assert!(matches!(
        store.get_view(first.id, view.id).await,
        Err(AppError::NotFound)
    ));
    let after = store.get_record(first.id, record.id).await.unwrap();
    assert_eq!(after.values, record.values);
    assert_eq!(after.revision, 1);
    assert!(matches!(
        store.list_views(999999).await,
        Err(AppError::NotFound)
    ));
}

#[tokio::test]
async fn duplicate_names_caps_and_concurrent_updates_preserve_the_winner() {
    let store = Store::open(":memory:").unwrap();
    let app = store.create_app(schema("Caps")).await.unwrap();
    let view = store.create_view(app.id, definition("same")).await.unwrap();
    assert!(matches!(
        store.create_view(app.id, definition("same")).await,
        Err(AppError::DuplicateViewName)
    ));
    let (a, b) = tokio::join!(
        store.update_view(app.id, view.id, 1, definition("A")),
        store.update_view(app.id, view.id, 1, definition("B"))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(AppError::Conflict)
    ));
    let other = store.create_app(schema("Other")).await.unwrap();
    store
        .create_view(other.id, definition("same"))
        .await
        .unwrap();
    for i in 1..49 {
        store
            .create_view(app.id, definition(&format!("View {i}")))
            .await
            .unwrap();
    }
    let (a, b) = tokio::join!(
        store.create_view(app.id, definition("last A")),
        store.create_view(app.id, definition("last B"))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(AppError::ViewLimit)
    ));
    assert_eq!(store.list_views(app.id).await.unwrap().len(), 50);
    let first = store.get_view(app.id, view.id).await.unwrap();
    store
        .update_view(
            app.id,
            view.id,
            first.revision,
            definition("Allowed at cap"),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn api_validation_unique_names_and_revisions_are_explicit() {
    let store = Store::open(":memory:").unwrap();
    let app_id = store.create_app(schema("HTTP")).await.unwrap().id;
    let app = router(store.clone());
    let path = format!("/api/v1/apps/{app_id}/views");
    for body in [
        json!({"name":"x","columns":[]}),
        json!({"name":"x","columns":["other"]}),
        json!({"name":"x","columns":["name"],"extra":1}),
        json!({"name":"x","columns":["name"],"filter":{"field":"done","value":0}}),
    ] {
        assert_eq!(
            request(&app, "POST", &path, body, Some("1"), None).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, view) = request(
        &app,
        "POST",
        &path,
        serde_json::to_value(definition("東京 '<img>")).unwrap(),
        Some("1"),
        Some(("origin", "http://localhost:8080")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(view["app_id"], app_id);
    assert_eq!(view["revision"], 1);
    let id = view["id"].as_i64().unwrap();
    let detail = format!("{path}/{id}");
    let (status, error) = request(
        &app,
        "POST",
        &path,
        view["definition"].clone(),
        Some("1"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "duplicate_view_name");
    assert_eq!(
        request(&app, "GET", &detail, Value::Null, None, None)
            .await
            .1,
        view
    );
    let update = json!({"revision":1,"definition":{"name":"Edited", "columns":["name"]}});
    assert_eq!(
        request(&app, "PUT", &detail, update.clone(), Some("1"), None)
            .await
            .1["revision"],
        2
    );
    assert_eq!(
        request(&app, "PUT", &detail, update, Some("1"), None)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{detail}?revision=1"),
            Value::Null,
            Some("1"),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{detail}?revision=2"),
            Value::Null,
            Some("1"),
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", &path, Value::Null, None, None).await.1,
        json!([])
    );
}

#[tokio::test]
async fn rejected_browser_mutations_leave_views_and_records_unchanged() {
    let store = Store::open(":memory:").unwrap();
    let app_id = store.create_app(schema("Boundary")).await.unwrap().id;
    let view = store
        .create_view(app_id, definition("Original"))
        .await
        .unwrap();
    let record = store
        .create_record(
            app_id,
            json!({"name":"keep","done":false})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    let app = router(store.clone());
    let base = format!("/api/v1/apps/{app_id}/views");
    for (marker, extra) in [
        (None, None),
        (Some("0"), None),
        (Some("1"), Some(("origin", "http://localhost:9999"))),
        (Some("1"), Some(("origin", "null"))),
        (Some("1"), Some(("sec-fetch-site", "cross-site"))),
    ] {
        for (method, path, body) in [
            (
                "POST",
                base.clone(),
                serde_json::to_value(definition("Intrusion")).unwrap(),
            ),
            (
                "PUT",
                format!("{base}/{}", view.id),
                json!({"revision":1,"definition":{"name":"Intrusion","columns":["name"]}}),
            ),
            (
                "DELETE",
                format!("{base}/{}?revision=1", view.id),
                Value::Null,
            ),
        ] {
            assert_eq!(
                request(&app, method, &path, body, marker, extra).await.0,
                StatusCode::FORBIDDEN
            );
        }
    }
    let after = store.get_view(app_id, view.id).await.unwrap();
    assert_eq!(after.revision, 1);
    assert_eq!(after.definition, view.definition);
    assert_eq!(store.list_views(app_id).await.unwrap().len(), 1);
    assert_eq!(
        store.get_record(app_id, record.id).await.unwrap().values,
        record.values
    );
}
