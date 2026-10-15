// SPDX-License-Identifier: Apache-2.0
use crate::AppError;
use business_apps_domain::{
    App, NewApp, Record, RecordFilter, Values, validate_app, validate_app_update,
    validate_record_filter, validate_values,
};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use std::{path::Path, time::Duration};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Connection) + Send>;
mod migrations;

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
        revision: row.get(3)?,
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
            "SELECT id, name, fields, revision FROM apps WHERE id = ?1",
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

// Both paged listing and complete export use literal per-app text matching.
const RECORD_SELECTION: &str = "
    SELECT id, app_id, revision, values_json FROM records
    WHERE app_id = ?1 AND id < ?2
    AND (?3 IS NULL OR EXISTS (
        SELECT 1 FROM json_each(records.values_json) AS entry
        WHERE entry.type = 'text' AND instr(lower(CAST(entry.value AS TEXT)), lower(?3)) > 0
    ))
    AND (?5 IS NULL OR
        (?6 = 'null' AND NOT EXISTS (
            SELECT 1 FROM json_each(records.values_json) AS cell
            WHERE cell.key = ?5 AND cell.type != 'null' AND NOT (cell.type = 'text' AND cell.value = '')
        )) OR EXISTS (
            SELECT 1 FROM json_each(records.values_json) AS cell
            WHERE cell.key = ?5 AND cell.value = json_extract(?6, '$')
            AND (cell.type = json_type(?6) OR
                (cell.type IN ('integer', 'real') AND json_type(?6) IN ('integer', 'real')))
        )
    ) ORDER BY id DESC LIMIT ?4
";

fn search_query(query: Option<String>) -> Result<Option<String>, AppError> {
    if query.as_ref().is_some_and(|q| q.chars().count() > 128) {
        return Err(AppError::Validation(
            "Search text exceeds 128 characters.".into(),
        ));
    }
    Ok(query.filter(|q| !q.is_empty()))
}

fn filter_parameters(
    app: &App,
    filter: Option<&RecordFilter>,
) -> Result<(Option<String>, Option<String>), AppError> {
    match filter {
        None => Ok((None, None)),
        Some(filter) => {
            validate_record_filter(&app.fields, filter)?;
            Ok((Some(filter.field.clone()), Some(filter.value.to_string())))
        }
    }
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(3))?;
        migrations::prepare(&mut connection)?;
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
            let mut statement = connection
                .prepare_cached("SELECT id, name, fields, revision FROM apps ORDER BY id DESC")?;
            Ok(statement
                .query_map([], app_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
    }

    pub async fn get_app(&self, app_id: i64) -> Result<App, AppError> {
        self.call(move |connection| find_app(connection, app_id))
            .await
    }

    pub async fn update_app(
        &self,
        app_id: i64,
        revision: i64,
        input: NewApp,
    ) -> Result<App, AppError> {
        if revision <= 0 {
            return Err(AppError::Validation("Revision must be positive.".into()));
        }
        self.call(move |connection| {
            let current = find_app(connection, app_id)?;
            if current.revision != revision { return Err(AppError::Conflict); }
            validate_app_update(&current, &input)?;
            let fields = serde_json::to_string(&input.fields)?;
            let changed = connection.execute(
                "UPDATE apps SET name = ?1, fields = ?2, revision = revision + 1 WHERE id = ?3 AND revision = ?4",
                params![input.name, fields, app_id, revision],
            )?;
            if changed == 0 { return Err(AppError::Conflict); }
            find_app(connection, app_id)
        }).await
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
        self.list_records_filtered(app_id, before, limit, query, None)
            .await
    }

    pub async fn list_records_filtered(
        &self,
        app_id: i64,
        before: Option<i64>,
        limit: usize,
        query: Option<String>,
        filter: Option<RecordFilter>,
    ) -> Result<RecordPage, AppError> {
        if !(1..=100).contains(&limit) || before.is_some_and(|id| id <= 0) {
            return Err(AppError::Validation(
                "Use a limit of 1–100 and a positive cursor.".into(),
            ));
        }
        let query = search_query(query)?;
        self.call(move |connection| {
            let app = find_app(connection, app_id)?;
            let (field, value) = filter_parameters(&app, filter.as_ref())?;
            let mut statement = connection.prepare_cached(RECORD_SELECTION)?;
            let mut records = statement
                .query_map(
                    params![
                        app_id,
                        before.unwrap_or(i64::MAX),
                        query,
                        limit as i64 + 1,
                        field,
                        value
                    ],
                    record_row,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let has_more = records.len() > limit;
            if has_more {
                records.pop();
            }
            let next_cursor = if has_more {
                records.last().map(|record| record.id)
            } else {
                None
            };
            Ok(RecordPage {
                records,
                next_cursor,
            })
        })
        .await
    }

    pub async fn export_csv(&self, app_id: i64, query: Option<String>) -> Result<String, AppError> {
        self.export_csv_filtered(app_id, query, None).await
    }

    pub async fn export_csv_filtered(
        &self,
        app_id: i64,
        query: Option<String>,
        filter: Option<RecordFilter>,
    ) -> Result<String, AppError> {
        let query = search_query(query)?;
        self.call(move |connection| {
            let transaction = connection.transaction()?;
            let app = find_app(&transaction, app_id)?;
            let (field, value) = filter_parameters(&app, filter.as_ref())?;
            let mut csv = crate::csv::Csv::new();
            let mut headers = vec![
                serde_json::Value::String("ID".into()),
                serde_json::Value::String("Revision".into()),
            ];
            headers.extend(
                app.fields
                    .iter()
                    .map(|field| serde_json::Value::String(field.label.clone())),
            );
            csv.row(&headers)?;
            {
                let mut statement = transaction.prepare(RECORD_SELECTION)?;
                let rows = statement.query_map(
                    params![
                        app_id,
                        i64::MAX,
                        query,
                        crate::csv::MAX_ROWS as i64 + 1,
                        field,
                        value
                    ],
                    record_row,
                )?;
                for (index, row) in rows.enumerate() {
                    if index == crate::csv::MAX_ROWS {
                        return Err(AppError::ExportLimit);
                    }
                    let record = row?;
                    let id = serde_json::Value::from(record.id);
                    let revision = serde_json::Value::from(record.revision);
                    csv.row(
                        [&id, &revision]
                            .into_iter()
                            .chain(app.fields.iter().map(|field| {
                                record
                                    .values
                                    .get(&field.key)
                                    .unwrap_or(&serde_json::Value::Null)
                            })),
                    )?;
                }
            }
            transaction.commit()?;
            Ok(csv.finish())
        })
        .await
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
