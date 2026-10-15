// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::{NewApp, RecordFilter, validate_record_filter};
use serde_json::json;

fn schema() -> NewApp {
    serde_json::from_value(json!({"name":"Filters", "fields":[
        {"key":"name","label":"Name","kind":"text","required":true},
        {"key":"note","label":"Note","kind":"textarea"},
        {"key":"amount","label":"Amount","kind":"number"},
        {"key":"done","label":"Done","kind":"boolean"},
        {"key":"due","label":"Due","kind":"date"},
        {"key":"status","label":"Status","kind":"select","options":["未対応","完了"]}
    ]}))
    .unwrap()
}

#[test]
fn filters_validate_each_field_type_and_accept_empty_queries_for_required_fields() {
    let fields = schema().fields;
    for (field, value) in [
        ("name", json!("東京 ' %_")),
        ("note", json!("a\nb")),
        ("amount", json!(0)),
        ("amount", json!(-1.5)),
        ("done", json!(false)),
        ("done", json!(true)),
        ("due", json!("2024-02-29")),
        ("status", json!("完了")),
    ] {
        validate_record_filter(
            &fields,
            &RecordFilter {
                field: field.into(),
                value,
            },
        )
        .unwrap();
    }
    for field in &fields {
        validate_record_filter(
            &fields,
            &RecordFilter {
                field: field.key.clone(),
                value: json!(null),
            },
        )
        .unwrap();
    }
    validate_record_filter(
        &fields,
        &RecordFilter {
            field: "name".into(),
            value: json!("😀".repeat(128)),
        },
    )
    .unwrap();
}

#[test]
fn filters_reject_unknown_fields_wrong_types_unsafe_numbers_and_invalid_choices() {
    let fields = schema().fields;
    for (field, value) in [
        ("missing", json!("x")),
        ("name' OR 1=1 --", json!("x")),
        ("name", json!(false)),
        ("name", json!("あ".repeat(129))),
        ("note", json!([])),
        ("amount", json!(false)),
        ("amount", json!("0")),
        ("amount", json!(9_007_199_254_740_992_i64)),
        ("done", json!(0)),
        ("due", json!("2023-02-29")),
        ("status", json!("other")),
        ("status", json!({"x":1})),
    ] {
        assert!(
            validate_record_filter(
                &fields,
                &RecordFilter {
                    field: field.into(),
                    value
                }
            )
            .is_err(),
            "field {field}"
        );
    }
}

#[test]
fn filter_shape_requires_field_and_value_and_rejects_unknown_properties() {
    for input in [
        json!({"field":"name"}),
        json!({"value":null}),
        json!({"field":"name","value":"x","op":"sql"}),
        json!([]),
    ] {
        assert!(serde_json::from_value::<RecordFilter>(input).is_err());
    }
}
