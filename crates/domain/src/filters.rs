// SPDX-License-Identifier: Apache-2.0
use crate::{Field, ValidationError, Values, invalid, validate_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One typed equality condition. Null selects missing, null, or empty text cells.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFilter {
    pub field: String,
    pub value: Value,
}

pub fn validate_record_filter(
    fields: &[Field],
    filter: &RecordFilter,
) -> Result<(), ValidationError> {
    let mut field = fields
        .iter()
        .find(|field| field.key == filter.field)
        .ok_or_else(|| invalid("Unknown filter field."))?
        .clone();
    if filter
        .value
        .as_str()
        .is_some_and(|text| text.chars().count() > 128)
    {
        return Err(invalid("Filter text exceeds 128 characters."));
    }
    // A query may select absent values even for a currently required field.
    field.required = false;
    let mut values = Values::new();
    values.insert(filter.field.clone(), filter.value.clone());
    validate_values(&[field], &values)
}
