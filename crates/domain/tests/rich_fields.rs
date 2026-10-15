// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::{NewApp, validate_app, validate_values};
use serde_json::{Value, json};

fn schema(kind: &str, required: bool, options: Value) -> NewApp {
    serde_json::from_value(json!({"name":"Tasks","fields":[
        {"key":"value","label":"Value","kind":kind,"required":required,"options":options}
    ]}))
    .unwrap()
}

#[test]
fn gregorian_dates_have_exact_format_and_real_days() {
    let app = schema("date", true, json!([]));
    validate_app(&app).unwrap();
    for date in ["0001-01-01", "2000-02-29", "2024-02-29", "9999-12-31"] {
        validate_values(&app.fields, json!({"value":date}).as_object().unwrap()).unwrap();
    }
    for date in [
        "",
        "0000-01-01",
        "1900-02-29",
        "2100-02-29",
        "2025-02-29",
        "2026-04-31",
        "2026-00-01",
        "2026-13-01",
        "2026-01-00",
        "2026-1-01",
        "2026-01-01T00:00:00Z",
        "２０２６-01-01",
        "éé-01-01",
    ] {
        assert!(
            validate_values(&app.fields, json!({"value":date}).as_object().unwrap()).is_err(),
            "{date}"
        );
    }
    for values in [json!({}), json!({"value":null}), json!({"value":20260101})] {
        assert!(validate_values(&app.fields, values.as_object().unwrap()).is_err());
    }
    let optional = schema("date", false, json!([]));
    for values in [json!({}), json!({"value":null})] {
        validate_values(&optional.fields, values.as_object().unwrap()).unwrap();
    }
}

#[test]
fn select_schema_and_values_are_bounded_and_exact() {
    let app = schema("select", true, json!(["未対応", "対応済み", "<img src=x>"]));
    validate_app(&app).unwrap();
    for value in ["未対応", "対応済み", "<img src=x>"] {
        validate_values(&app.fields, json!({"value":value}).as_object().unwrap()).unwrap();
    }
    for value in [json!(""), json!("未登録"), json!(["未対応"]), json!(null)] {
        assert!(validate_values(&app.fields, json!({"value":value}).as_object().unwrap()).is_err());
    }
    for options in [
        json!([]),
        json!([" "]),
        json!(["a", "a"]),
        json!([" a"]),
        json!(["a\nb"]),
        json!(["x".repeat(81)]),
        json!((0..51).map(|i| i.to_string()).collect::<Vec<_>>()),
    ] {
        assert!(validate_app(&schema("select", false, options)).is_err());
    }
    for kind in ["text", "textarea", "date", "number", "boolean"] {
        assert!(validate_app(&schema(kind, false, json!(["invalid"]))).is_err());
    }
    let optional = schema("select", false, json!(["a"]));
    validate_values(&optional.fields, json!({"value":null}).as_object().unwrap()).unwrap();
}

#[test]
fn multiline_text_preserves_plain_content_and_limits() {
    let app = schema("textarea", true, json!([]));
    for text in [
        "line 1\n日本語\n<script>literal</script>".into(),
        "客".repeat(4_000),
    ] {
        validate_values(&app.fields, json!({"value":text}).as_object().unwrap()).unwrap();
    }
    for value in [json!(" \n "), json!("客".repeat(4_001)), json!(3)] {
        assert!(validate_values(&app.fields, json!({"value":value}).as_object().unwrap()).is_err());
    }
}
