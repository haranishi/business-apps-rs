// SPDX-License-Identifier: Apache-2.0
use crate::AppError;
use rusqlite::{Connection, TransactionBehavior};

const APPLICATION_ID: i64 = 0x42415253;

pub(super) fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    // Inspect and migrate under the same write lock, including concurrent opens.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let owner: i64 = transaction.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if version == 0 && owner == 0 {
        let tables: i64 = transaction.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;
        if tables != 0 {
            return Err(AppError::Unavailable);
        }
        transaction.execute_batch(&format!(
            "
            CREATE TABLE apps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                fields TEXT NOT NULL CHECK(json_valid(fields)),
                revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
            );
            CREATE TABLE records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_id INTEGER NOT NULL REFERENCES apps(id),
                revision INTEGER NOT NULL CHECK(revision > 0),
                values_json TEXT NOT NULL CHECK(json_valid(values_json))
            );
            CREATE INDEX records_by_app_id ON records(app_id, id DESC);
            PRAGMA application_id = {APPLICATION_ID};
        "
        ))?;
    } else if owner == APPLICATION_ID && matches!(version, 1 | 2) {
        transaction.execute_batch(
            "ALTER TABLE apps ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0);",
        )?;
    } else if owner != APPLICATION_ID || !matches!(version, 3 | 4) {
        return Err(AppError::Unavailable);
    }
    if version != 4 {
        transaction.execute_batch(
            "CREATE TABLE record_views (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            app_id INTEGER NOT NULL REFERENCES apps(id),
            revision INTEGER NOT NULL CHECK(revision > 0),
            name TEXT NOT NULL,
            definition TEXT NOT NULL CHECK(json_valid(definition)),
            UNIQUE(app_id, name)
        );",
        )?;
        transaction.pragma_update(None, "user_version", 4)?;
    }
    transaction.commit()?;
    Ok(())
}
