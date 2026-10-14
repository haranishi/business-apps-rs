// SPDX-License-Identifier: Apache-2.0
use crate::AppError;
use business_apps_domain::{App, NewApp, Record, Values, validate_app, validate_values};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use std::{path::Path, time::Duration};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Connection) + Send>;
const APPLICATION_ID: i64 = 0x42415253;

#[derive(Clone)]
pub struct Store {
    sender: mpsc::Sender<Job>,
}

#[derive(Debug, Serialize)]
pub struct RecordPage {
    pub records: Vec<Record>,
    pub next_cursor: Option<i64>,
}

fn decode_json<T: serde::de::DeserializeOwned>(
    row: &Row<'_>,
    column: usize,
) -> rusqlite::Result<T> {
    let text: String = row.get(column)?;
    serde_json::from_str(&text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn app_row(row: &Row<'_>) -> rusqlite::Result<App> {
    Ok(App {
        id: row.get(0)?,
        name: row.get(1)?,
        fields: decode_json(row, 2)?,
    })
}

fn record_row(row: &Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get(0)?,
        app_id: row.get(1)?,
        revision: row.get(2)?,
        values: decode_json(row, 3)?,
    })
}

fn find_app(connection: &Connection, id: i64) -> Result<App, AppError> {
    connection
        .query_row(
            "SELECT id, name, fields FROM apps WHERE id = ?1",
            [id],
            app_row,
        )
        .optional()?
        .ok_or(AppError::NotFound)
}

fn find_record(connection: &Connection, app_id: i64, id: i64) -> Result<Record, AppError> {
    connection
        .query_row(
            "SELECT id, app_id, revision, values_json FROM records WHERE app_id = ?1 AND id = ?2",
            params![app_id, id],
            record_row,
        )
        .optional()?
        .ok_or(AppError::NotFound)
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(3))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let owner: i64 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        if version == 0 && owner == 0 {
            let tables: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0))?;
            if tables != 0 {
                return Err(AppError::Unavailable);
            }
            let transaction = connection.transaction()?;
            transaction.execute_batch(&format!(
                "
                CREATE TABLE apps (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    fields TEXT NOT NULL CHECK(json_valid(fields))
                );
                CREATE TABLE records (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    app_id INTEGER NOT NULL REFERENCES apps(id),
                    revision INTEGER NOT NULL CHECK(revision > 0),
                    values_json TEXT NOT NULL CHECK(json_valid(values_json))
                );
                CREATE INDEX records_by_app_id ON records(app_id, id DESC);
                PRAGMA application_id = {APPLICATION_ID};
                PRAGMA user_version = 1;
            "
            ))?;
            transaction.commit()?;
        } else if owner != APPLICATION_ID || version != 1 {
            return Err(AppError::Unavailable);
        }
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        // Keep SQLite's durable FULL synchronous default; do not trade durability for benchmark numbers.
        let (sender, mut receiver) = mpsc::channel::<Job>(128);
        std::thread::Builder::new()
            .name("business-apps-sqlite".into())
            .spawn(move || {
                while let Some(job) = receiver.blocking_recv() {
                    job(&mut connection);
                }
            })
            .map_err(|_| AppError::Unavailable)?;
        Ok(Self { sender })
    }

    pub async fn call<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let (send, receive) = oneshot::channel();
        self.sender
            .send(Box::new(move |connection| {
                let _ = send.send(operation(connection));
            }))
            .await
            .map_err(|_| AppError::Unavailable)?;
        receive.await.map_err(|_| AppError::Unavailable)?
    }

    pub async fn create_app(&self, input: NewApp) -> Result<App, AppError> {
        validate_app(&input)?;
        self.call(move |connection| {
            let fields = serde_json::to_string(&input.fields)?;
            connection.execute(
                "INSERT INTO apps(name, fields) VALUES (?1, ?2)",
                params![input.name, fields],
            )?;
            find_app(connection, connection.last_insert_rowid())
        })
        .await
    }

    pub async fn list_apps(&self) -> Result<Vec<App>, AppError> {
        self.call(|connection| {
            let mut statement =
                connection.prepare_cached("SELECT id, name, fields FROM apps ORDER BY id DESC")?;
            Ok(statement
                .query_map([], app_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
    }

    pub async fn create_record(&self, app_id: i64, values: Values) -> Result<Record, AppError> {
        self.call(move |connection| {
            validate_values(&find_app(connection, app_id)?.fields, &values)?;
            let serialized = serde_json::to_string(&values)?;
            connection.execute(
                "INSERT INTO records(app_id, revision, values_json) VALUES (?1, 1, ?2)",
                params![app_id, serialized],
            )?;
            find_record(connection, app_id, connection.last_insert_rowid())
        })
        .await
    }

    pub async fn get_record(&self, app_id: i64, id: i64) -> Result<Record, AppError> {
        self.call(move |connection| find_record(connection, app_id, id))
            .await
    }

    pub async fn list_records(
        &self,
        app_id: i64,
        before: Option<i64>,
        limit: usize,
        query: Option<String>,
    ) -> Result<RecordPage, AppError> {
        if !(1..=100).contains(&limit) || before.is_some_and(|id| id <= 0) {
            return Err(AppError::Validation(
                "Use a limit of 1–100 and a positive cursor.".into(),
            ));
        }
        if query.as_ref().is_some_and(|q| q.chars().count() > 128) {
            return Err(AppError::Validation(
                "Search text exceeds 128 characters.".into(),
            ));
        }
        let query = query.filter(|q| !q.is_empty());
        self.call(move |connection| {
            find_app(connection, app_id)?;
            let mut statement = connection.prepare_cached("
                SELECT id, app_id, revision, values_json FROM records
                WHERE app_id = ?1 AND id < ?2
                AND (?3 IS NULL OR EXISTS (
                    SELECT 1 FROM json_each(records.values_json) AS entry
                    WHERE entry.type = 'text' AND instr(lower(CAST(entry.value AS TEXT)), lower(?3)) > 0
                )) ORDER BY id DESC LIMIT ?4
            ")?;
            let mut records = statement.query_map(params![app_id, before.unwrap_or(i64::MAX), query, limit as i64 + 1], record_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let has_more = records.len() > limit;
            if has_more { records.pop(); }
            let next_cursor = if has_more { records.last().map(|record| record.id) } else { None };
            Ok(RecordPage { records, next_cursor })
        }).await
    }

    pub async fn update_record(
        &self,
        app_id: i64,
        id: i64,
        revision: i64,
        values: Values,
    ) -> Result<Record, AppError> {
        if revision <= 0 {
            return Err(AppError::Validation("Revision must be positive.".into()));
        }
        self.call(move |connection| {
            let current = find_record(connection, app_id, id)?;
            if current.revision != revision { return Err(AppError::Conflict); }
            validate_values(&find_app(connection, app_id)?.fields, &values)?;
            let serialized = serde_json::to_string(&values)?;
            let changed = connection.execute(
                "UPDATE records SET values_json = ?1, revision = revision + 1 WHERE app_id = ?2 AND id = ?3 AND revision = ?4",
                params![serialized, app_id, id, revision])?;
            if changed == 0 {
                find_record(connection, app_id, id)?;
                return Err(AppError::Conflict);
            }
            find_record(connection, app_id, id)
        }).await
    }

    pub async fn delete_record(&self, app_id: i64, id: i64, revision: i64) -> Result<(), AppError> {
        if revision <= 0 {
            return Err(AppError::Validation("Revision must be positive.".into()));
        }
        self.call(move |connection| {
            let changed = connection.execute(
                "DELETE FROM records WHERE app_id = ?1 AND id = ?2 AND revision = ?3",
                params![app_id, id, revision],
            )?;
            if changed == 0 {
                find_record(connection, app_id, id)?;
                return Err(AppError::Conflict);
            }
            Ok(())
        })
        .await
    }
}
