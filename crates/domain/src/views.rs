// SPDX-License-Identifier: Apache-2.0
use crate::{Field, RecordFilter, ValidationError, invalid, valid_label, validate_record_filter};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_VIEWS_PER_APP: usize = 50;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ViewDefinition {
    pub name: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub filter: Option<RecordFilter>,
    pub columns: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedView {
    pub id: i64,
    pub app_id: i64,
    pub revision: i64,
    pub definition: ViewDefinition,
}

pub fn validate_view(fields: &[Field], view: &ViewDefinition) -> Result<(), ValidationError> {
    if !valid_label(&view.name) || view.name.trim() != view.name {
        return Err(invalid(
            "View name must contain 1–80 characters without surrounding whitespace.",
        ));
    }
    if view.query.chars().count() > 128 {
        return Err(invalid("View search text exceeds 128 characters."));
    }
    let mut keys = HashSet::new();
    if view.columns.is_empty()
        || view.columns.len() > 32
        || view
            .columns
            .iter()
            .any(|key| !keys.insert(key) || !fields.iter().any(|field| &field.key == key))
    {
        return Err(invalid(
            "Choose 1–32 distinct fields from this app for the view.",
        ));
    }
    if let Some(filter) = &view.filter {
        validate_record_filter(fields, filter)?;
    }
    Ok(())
}
