// SPDX-License-Identifier: Apache-2.0
//! Browser workflows compiled from Rust to WebAssembly.

#[cfg(any(target_arch = "wasm32", test))]
use business_apps_domain::{FieldKind, Values};
#[cfg(any(target_arch = "wasm32", test))]
use serde_json::Value;

/// Browser textareas normalize CRLF and CR on display. Preserve the saved
/// representation when the editor did not change its displayed text.
#[cfg(any(target_arch = "wasm32", test))]
fn edited_value(kind: FieldKind, text: &str, checked: bool, original: Option<&Value>) -> Value {
    if kind == FieldKind::Textarea
        && let Some(Value::String(prior)) = original
        && normalize_line_endings(prior) == text
    {
        return Value::String(prior.clone());
    }
    match kind {
        FieldKind::Boolean => Value::Bool(checked),
        FieldKind::Number if text.is_empty() => Value::Null,
        FieldKind::Number => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number),
        FieldKind::Date | FieldKind::Select if text.is_empty() => Value::Null,
        _ => Value::String(text.into()),
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Begin with every loaded value, including fields from a newer schema that an
/// older tab cannot render. Only visible controls replace their own keys.
#[cfg(any(target_arch = "wasm32", test))]
fn merge_edited_values<'a>(
    original: Option<&Values>,
    edits: impl IntoIterator<Item = (&'a str, FieldKind, String, bool)>,
) -> Values {
    let mut values = original.cloned().unwrap_or_default();
    for (key, kind, text, checked) in edits {
        let value = edited_value(kind, &text, checked, original.and_then(|map| map.get(key)));
        values.insert(key.into(), value);
    }
    values
}

#[cfg(target_arch = "wasm32")]
mod builder;
#[cfg(target_arch = "wasm32")]
mod dom;
#[cfg(target_arch = "wasm32")]
mod layout;
#[cfg(target_arch = "wasm32")]
mod records;
#[cfg(target_arch = "wasm32")]
mod runtime;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    runtime::start()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn editing_an_old_schema_preserves_unrendered_new_values() {
        let original = json!({"name":"Before", "added":"Keep", "__proto__":"literal"});
        let values = merge_edited_values(
            original.as_object(),
            [("name", FieldKind::Text, "After".into(), false)],
        );
        assert_eq!(
            values,
            json!({"name":"After", "added":"Keep", "__proto__":"literal"})
                .as_object()
                .unwrap()
                .clone()
        );
    }

    #[test]
    fn unchanged_multiline_content_keeps_api_line_endings() {
        let original = json!("first\r\nsecond\rthird");
        assert_eq!(
            edited_value(
                FieldKind::Textarea,
                "first\nsecond\nthird",
                false,
                Some(&original)
            ),
            original
        );
        assert_eq!(
            edited_value(FieldKind::Textarea, "changed\ntext", false, Some(&original)),
            json!("changed\ntext")
        );
    }

    #[test]
    fn typed_controls_keep_zero_false_and_optional_null() {
        assert_eq!(
            edited_value(FieldKind::Number, "0", false, None),
            json!(0.0)
        );
        assert_eq!(
            edited_value(FieldKind::Boolean, "", false, None),
            json!(false)
        );
        for kind in [FieldKind::Number, FieldKind::Date, FieldKind::Select] {
            assert_eq!(edited_value(kind, "", false, None), Value::Null);
        }
        assert_eq!(
            edited_value(FieldKind::Select, "\u{feff}Open", false, None),
            json!("\u{feff}Open")
        );
    }
}
