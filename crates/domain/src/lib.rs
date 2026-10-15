// SPDX-License-Identifier: Apache-2.0
//! Typed business schemas and values, independent of HTTP and persistence.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashSet, fmt};

pub type Values = Map<String, Value>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewApp {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    Boolean,
    Date,
    Select,
    Textarea,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct App {
    pub id: i64,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: i64,
    pub app_id: i64,
    pub revision: i64,
    pub values: Values,
}

#[derive(Debug)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ValidationError {}

fn invalid(message: impl Into<String>) -> ValidationError {
    ValidationError(message.into())
}

fn valid_label(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 80
}

pub fn validate_app(app: &NewApp) -> Result<(), ValidationError> {
    if !valid_label(&app.name) {
        return Err(invalid("App name must contain 1–80 characters."));
    }
    if !(1..=32).contains(&app.fields.len()) {
        return Err(invalid("An app must contain 1–32 fields."));
    }
    let mut keys = HashSet::new();
    for field in &app.fields {
        let mut chars = field.key.chars();
        if field.key.len() > 40
            || !chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(invalid(
                "Field keys must start with a letter and contain 1–40 ASCII letters, digits, or underscores.",
            ));
        }
        if !keys.insert(&field.key) {
            return Err(invalid(format!("Duplicate field key: {}", field.key)));
        }
        if !valid_label(&field.label) {
            return Err(invalid("Field labels must contain 1–80 characters."));
        }
        if field.kind == FieldKind::Select {
            let mut choices = HashSet::new();
            if !(1..=50).contains(&field.options.len())
                || field.options.iter().any(|option| {
                    !valid_label(option)
                        || option.trim() != option
                        || option.contains(['\r', '\n'])
                        || !choices.insert(option)
                })
            {
                return Err(invalid(
                    "Select fields need 1–50 distinct, trimmed options of 1–80 characters without line breaks.",
                ));
            }
        } else if !field.options.is_empty() {
            return Err(invalid("Only select fields can have options."));
        }
    }
    Ok(())
}

pub fn validate_values(fields: &[Field], values: &Values) -> Result<(), ValidationError> {
    for key in values.keys() {
        if !fields.iter().any(|field| &field.key == key) {
            return Err(invalid(format!("Unknown field: {key}")));
        }
    }
    for field in fields {
        let value = values.get(&field.key);
        if value.is_none() || value.is_some_and(Value::is_null) {
            if field.required {
                return Err(invalid(format!("{} is required.", field.label)));
            }
            continue;
        }
        let value = value.expect("missing values handled above");
        match field.kind {
            FieldKind::Text | FieldKind::Textarea => {
                let text = value
                    .as_str()
                    .ok_or_else(|| invalid(format!("{} must be text.", field.label)))?;
                if text.chars().count() > 4_000 {
                    return Err(invalid(format!(
                        "{} exceeds 4,000 characters.",
                        field.label
                    )));
                }
                if field.required && text.trim().is_empty() {
                    return Err(invalid(format!("{} is required.", field.label)));
                }
            }
            FieldKind::Date => {
                if !value.as_str().is_some_and(valid_date) {
                    return Err(invalid(format!(
                        "{} must be a valid YYYY-MM-DD date (0001–9999).",
                        field.label
                    )));
                }
            }
            FieldKind::Select => {
                if !value
                    .as_str()
                    .is_some_and(|text| field.options.iter().any(|option| option == text))
                {
                    return Err(invalid(format!(
                        "{} must be one of its configured choices.",
                        field.label
                    )));
                }
            }
            FieldKind::Number => {
                let number = value
                    .as_number()
                    .ok_or_else(|| invalid(format!("{} must be a number.", field.label)))?;
                const MAX_SAFE: i64 = 9_007_199_254_740_991;
                let safe = if let Some(integer) = number.as_i64() {
                    (-MAX_SAFE..=MAX_SAFE).contains(&integer)
                } else if let Some(integer) = number.as_u64() {
                    integer <= MAX_SAFE as u64
                } else {
                    number
                        .as_f64()
                        .is_some_and(|float| float.is_finite() && float.abs() <= MAX_SAFE as f64)
                };
                if !safe {
                    return Err(invalid(format!(
                        "{} must be within the browser-safe numeric range ±9,007,199,254,740,991.",
                        field.label
                    )));
                }
            }
            FieldKind::Boolean if !value.is_boolean() => {
                return Err(invalid(format!("{} must be a boolean.", field.label)));
            }
            _ => {}
        }
    }
    Ok(())
}

fn valid_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, byte)| i != 4 && i != 7 && !byte.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = text[..4].parse().expect("checked ASCII digits");
    let month: u32 = text[5..7].parse().expect("checked ASCII digits");
    let day: u32 = text[8..].parse().expect("checked ASCII digits");
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    year >= 1 && day >= 1 && day <= days
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> NewApp {
        serde_json::from_value(json!({"name":"Inventory","fields":[
            {"key":"name","label":"Name","kind":"text","required":true},
            {"key":"stock","label":"Stock","kind":"number","required":true},
            {"key":"active","label":"Active","kind":"boolean","required":true},
            {"key":"note","label":"Note","kind":"text"}
        ]}))
        .unwrap()
    }

    #[test]
    fn zero_and_false_are_valid_required_values() {
        let values = json!({"name":"品物","stock":0,"active":false,"note":null});
        validate_values(&schema().fields, values.as_object().unwrap()).unwrap();
    }

    #[test]
    fn missing_blank_wrong_type_and_unknown_values_are_rejected() {
        for values in [
            json!({"name":"x","stock":0}),
            json!({"name":"  ","stock":0,"active":true}),
            json!({"name":"x","stock":"0","active":true}),
            json!({"name":"x","stock":0,"active":0}),
            json!({"name":"x","stock":0,"active":true,"extra":1}),
            json!({"name":null,"stock":0,"active":true}),
            json!({"name":"x".repeat(4_001),"stock":0,"active":true}),
        ] {
            assert!(
                validate_values(&schema().fields, values.as_object().unwrap()).is_err(),
                "{values}"
            );
        }
    }

    #[test]
    fn invalid_and_duplicate_schemas_are_rejected() {
        let mut app = schema();
        validate_app(&app).unwrap();
        app.fields.push(app.fields[0].clone());
        assert!(validate_app(&app).is_err());
        app.fields.pop();
        for key in ["", "a-b", "1name", "名前", "x; DROP TABLE apps"] {
            app.fields[0].key = key.into();
            assert!(validate_app(&app).is_err());
        }
        app.fields = vec![];
        assert!(validate_app(&app).is_err());
        app.name = " ".into();
        assert!(validate_app(&app).is_err());
    }

    #[test]
    fn unsafe_numbers_are_rejected_instead_of_silently_rounded() {
        let fields = schema().fields;
        for quantity in [
            json!(9_007_199_254_740_992_i64),
            json!(-9_007_199_254_740_992_i64),
            json!(u64::MAX),
            json!(1e20),
        ] {
            let values = json!({"name":"x","stock":quantity,"active":false});
            assert!(validate_values(&fields, values.as_object().unwrap()).is_err());
        }
        for quantity in [
            json!(9_007_199_254_740_991_i64),
            json!(-9_007_199_254_740_991_i64),
            json!(12.5),
        ] {
            let values = json!({"name":"x","stock":quantity,"active":false});
            validate_values(&fields, values.as_object().unwrap()).unwrap();
        }
    }

    #[test]
    fn unicode_length_is_counted_in_characters() {
        let mut app = schema();
        app.name = "客".repeat(80);
        validate_app(&app).unwrap();
        app.name.push('客');
        assert!(validate_app(&app).is_err());
    }
}
