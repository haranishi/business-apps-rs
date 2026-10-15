// SPDX-License-Identifier: Apache-2.0
use super::{Store, decode_json, find_app};
use crate::AppError;
use business_apps_domain::{MAX_VIEWS_PER_APP, SavedView, ViewDefinition, validate_view};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

fn view_row(row: &Row<'_>) -> rusqlite::Result<SavedView> {
    Ok(SavedView {
        id: row.get(0)?,
        app_id: row.get(1)?,
        revision: row.get(2)?,
        definition: decode_json(row, 3)?,
    })
}
fn find_view(connection: &Connection, app_id: i64, id: i64) -> Result<SavedView, AppError> {
    connection.query_row("SELECT id, app_id, revision, definition FROM record_views WHERE app_id = ?1 AND id = ?2", params![app_id, id], view_row).optional()?.ok_or(AppError::NotFound)
}
fn check_name(
    connection: &Connection,
    app_id: i64,
    name: &str,
    except: Option<i64>,
) -> Result<(), AppError> {
    let duplicate: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM record_views WHERE app_id = ?1 AND name = ?2 AND (?3 IS NULL OR id != ?3))", params![app_id, name, except], |row| row.get(0))?;
    if duplicate {
        return Err(AppError::DuplicateViewName);
    }
    Ok(())
}

impl Store {
    pub async fn list_views(&self, app_id: i64) -> Result<Vec<SavedView>, AppError> {
        self.call(move |connection| {
            find_app(connection, app_id)?;
            let mut statement = connection.prepare_cached("SELECT id, app_id, revision, definition FROM record_views WHERE app_id = ?1 ORDER BY id DESC")?;
            Ok(statement.query_map([app_id], view_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await
    }

    pub async fn get_view(&self, app_id: i64, id: i64) -> Result<SavedView, AppError> {
        self.call(move |connection| find_view(connection, app_id, id))
            .await
    }

    pub async fn create_view(
        &self,
        app_id: i64,
        definition: ViewDefinition,
    ) -> Result<SavedView, AppError> {
        self.call(move |connection| {
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            validate_view(&find_app(&transaction, app_id)?.fields, &definition)?;
            check_name(&transaction, app_id, &definition.name, None)?;
            let count: i64 = transaction.query_row("SELECT count(*) FROM record_views WHERE app_id = ?1", [app_id], |row| row.get(0))?;
            if count >= MAX_VIEWS_PER_APP as i64 { return Err(AppError::ViewLimit); }
            let serialized = serde_json::to_string(&definition)?;
            transaction.execute("INSERT INTO record_views(app_id, revision, name, definition) VALUES (?1, 1, ?2, ?3)", params![app_id, definition.name, serialized])?;
            let view = find_view(&transaction, app_id, transaction.last_insert_rowid())?;
            transaction.commit()?;
            Ok(view)
        }).await
    }

    pub async fn update_view(
        &self,
        app_id: i64,
        id: i64,
        revision: i64,
        definition: ViewDefinition,
    ) -> Result<SavedView, AppError> {
        if revision <= 0 {
            return Err(AppError::Validation("Revision must be positive.".into()));
        }
        self.call(move |connection| {
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let current = find_view(&transaction, app_id, id)?;
            if current.revision != revision { return Err(AppError::Conflict); }
            validate_view(&find_app(&transaction, app_id)?.fields, &definition)?;
            check_name(&transaction, app_id, &definition.name, Some(id))?;
            let serialized = serde_json::to_string(&definition)?;
            let changed = transaction.execute("UPDATE record_views SET name = ?1, definition = ?2, revision = revision + 1 WHERE app_id = ?3 AND id = ?4 AND revision = ?5", params![definition.name, serialized, app_id, id, revision])?;
            if changed == 0 { return Err(AppError::Conflict); }
            let view = find_view(&transaction, app_id, id)?;
            transaction.commit()?;
            Ok(view)
        }).await
    }

    pub async fn delete_view(&self, app_id: i64, id: i64, revision: i64) -> Result<(), AppError> {
        if revision <= 0 {
            return Err(AppError::Validation("Revision must be positive.".into()));
        }
        self.call(move |connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let current = find_view(&transaction, app_id, id)?;
            if current.revision != revision {
                return Err(AppError::Conflict);
            }
            let changed = transaction.execute(
                "DELETE FROM record_views WHERE app_id = ?1 AND id = ?2 AND revision = ?3",
                params![app_id, id, revision],
            )?;
            if changed == 0 {
                return Err(AppError::Conflict);
            }
            transaction.commit()?;
            Ok(())
        })
        .await
    }
}
