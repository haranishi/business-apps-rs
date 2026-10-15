// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::{NewApp, ViewDefinition};
use business_apps_server::{AppError, store::Store};
use rusqlite::Connection;
use serde_json::json;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static DATABASE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "business-views-{}-{}-{}.sqlite",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                .as_nanos(),
            DATABASE_COUNTER.fetch_add(1, Ordering::Relaxed)
            )))
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
    serde_json::from_value(json!({"name":"Migration", "fields":[{"key":"name","label":"Name","kind":"text"},{"key":"done","label":"Done","kind":"boolean"}]})).unwrap()
}
fn definition(name: &str) -> ViewDefinition {
    serde_json::from_value(
        json!({"name":name,"columns":["done","name"],"filter":{"field":"done","value":false}}),
    )
    .unwrap()
}
fn legacy(db: &Database) -> (String, String) {
    let connection = Connection::open(&db.0).unwrap();
    connection.execute_batch("CREATE TABLE apps(id INTEGER PRIMARY KEY AUTOINCREMENT,name TEXT NOT NULL, fields TEXT NOT NULL,revision INTEGER NOT NULL); CREATE TABLE records(id INTEGER PRIMARY KEY AUTOINCREMENT,app_id INTEGER NOT NULL REFERENCES apps(id),revision INTEGER NOT NULL,values_json TEXT NOT NULL); CREATE INDEX records_by_app_id ON records(app_id,id DESC); PRAGMA application_id=1111577171; PRAGMA user_version=3;").unwrap();
    let fields = serde_json::to_string(&schema().fields).unwrap();
    let values = json!({"name":"東京\r\n' %_", "done":false}).to_string();
    connection
        .execute("INSERT INTO apps VALUES(41,'Legacy',?1,11)", [&fields])
        .unwrap();
    connection
        .execute("INSERT INTO records VALUES(83,41,7,?1)", [&values])
        .unwrap();
    (fields, values)
}

#[tokio::test]
async fn format_three_upgrades_atomically_and_views_records_and_schemas_reopen() {
    let db = Database::new();
    let (fields, values) = legacy(&db);
    let store = Store::open(&db.0).unwrap();
    assert!(store.list_views(41).await.unwrap().is_empty());
    let view = store.create_view(41, definition("未完了")).await.unwrap();
    store
        .call(move |connection| {
            assert_eq!(
                connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?,
                4
            );
            assert_eq!(
                connection.query_row("SELECT fields FROM apps WHERE id=41", [], |row| row
                    .get::<_, String>(0))?,
                fields
            );
            assert_eq!(
                connection
                    .query_row("SELECT values_json FROM records WHERE id=83", [], |row| row
                        .get::<_, String>(0))?,
                values
            );
            Ok(())
        })
        .await
        .unwrap();
    drop(store);
    let reopened = Store::open(&db.0).unwrap();
    assert_eq!(reopened.get_app(41).await.unwrap().revision, 11);
    assert_eq!(reopened.get_record(41, 83).await.unwrap().revision, 7);
    assert_eq!(
        reopened.get_view(41, view.id).await.unwrap().definition,
        view.definition
    );
    assert_eq!(reopened.get_view(41, view.id).await.unwrap().revision, 1);
    let record = reopened
        .list_records_filtered(
            41,
            None,
            30,
            Some(view.definition.query),
            view.definition.filter,
        )
        .await
        .unwrap();
    assert_eq!(record.records[0].id, 83);
}

#[test]
fn failed_or_future_migration_does_not_modify_the_original_database() {
    for version in [3, 5] {
        let db = Database::new();
        let (_, values) = legacy(&db);
        let connection = Connection::open(&db.0).unwrap();
        if version == 3 {
            connection.execute_batch("CREATE TABLE record_views(original TEXT); INSERT INTO record_views VALUES('keep');").unwrap();
        }
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        drop(connection);
        assert!(matches!(Store::open(&db.0), Err(AppError::Unavailable)));
        let connection = Connection::open(&db.0).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            version
        );
        assert_eq!(
            connection
                .query_row("SELECT values_json FROM records WHERE id=83", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            values
        );
        if version == 3 {
            assert_eq!(
                connection
                    .query_row("SELECT original FROM record_views", [], |row| row
                        .get::<_, String>(0))
                    .unwrap(),
                "keep"
            );
        }
    }
}

#[tokio::test]
async fn independent_connections_share_the_cap_and_optimistic_revision_boundary() {
    let db = Database::new();
    let first = Store::open(&db.0).unwrap();
    let app = first.create_app(schema()).await.unwrap();
    let mut target = None;
    for index in 0..49 {
        target = Some(
            first
                .create_view(app.id, definition(&format!("View {index}")))
                .await
                .unwrap(),
        );
    }
    let second = Store::open(&db.0).unwrap();
    let (a, b) = tokio::join!(
        first.create_view(app.id, definition("A")),
        second.create_view(app.id, definition("B"))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(AppError::ViewLimit)
    ));
    assert_eq!(first.list_views(app.id).await.unwrap().len(), 50);
    let target = target.unwrap();
    let (a, b) = tokio::join!(
        first.update_view(app.id, target.id, 1, definition("Updated A")),
        second.update_view(app.id, target.id, 1, definition("Updated B"))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(AppError::Conflict)
    ));
}

#[tokio::test]
async fn simultaneous_opens_observe_one_complete_migration() {
    let db = Database::new();
    legacy(&db);
    let a = db.0.clone();
    let b = db.0.clone();
    let first = std::thread::spawn(move || Store::open(a));
    let second = std::thread::spawn(move || Store::open(b));
    let first = first.join().unwrap().unwrap();
    let second = second.join().unwrap().unwrap();
    assert!(first.list_views(41).await.unwrap().is_empty());
    assert!(second.list_views(41).await.unwrap().is_empty());
    assert_eq!(first.get_record(41, 83).await.unwrap().revision, 7);
}
