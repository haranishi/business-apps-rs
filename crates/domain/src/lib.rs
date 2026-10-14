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
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    Boolean,
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
            FieldKind::Text => {
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
            FieldKind::Number if !value.is_number() => {
                return Err(invalid(format!("{} must be a number.", field.label)));
            }
            FieldKind::Boolean if !value.is_boolean() => {
                return Err(invalid(format!("{} must be a boolean.", field.label)));
            }
            _ => {}
        }
    }
    Ok(())
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
    fn unicode_length_is_counted_in_characters() {
        let mut app = schema();
        app.name = "客".repeat(80);
        validate_app(&app).unwrap();
        app.name.push('客');
        assert!(validate_app(&app).is_err());
    }
}
