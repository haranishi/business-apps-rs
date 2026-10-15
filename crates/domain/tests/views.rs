// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::{NewApp, ViewDefinition, validate_view};
use serde_json::json;

fn schema() -> NewApp {
    serde_json::from_value(json!({"name":"Tasks", "fields":[
        {"key":"name","label":"Name","kind":"text","required":true},
        {"key":"done","label":"Done","kind":"boolean"}
    ]}))
    .unwrap()
}
fn definition() -> ViewDefinition {
    serde_json::from_value(json!({"name":"未対応", "query":"東京", "filter":{"field":"done","value":false}, "columns":["done","name"]})).unwrap()
}

#[test]
fn views_preserve_typed_conditions_column_order_and_literal_names() {
    let mut view = definition();
    validate_view(&schema().fields, &view).unwrap();
    view.name = "<img src=x> ' %_".into();
    view.query = "😀".repeat(128);
    validate_view(&schema().fields, &view).unwrap();
    let round_trip: ViewDefinition =
        serde_json::from_value(serde_json::to_value(&view).unwrap()).unwrap();
    assert_eq!(round_trip, view);
    view.filter.as_mut().unwrap().value = json!(null);
    validate_view(&schema().fields, &view).unwrap();
}

#[test]
fn views_reject_invalid_names_queries_columns_and_filter_values() {
    let fields = schema().fields;
    for name in ["".into(), " ".into(), " padded ".into(), "あ".repeat(81)] {
        let mut view = definition();
        view.name = name;
        assert!(validate_view(&fields, &view).is_err());
    }
    for columns in [
        vec![],
        vec!["name".into(), "name".into()],
        vec!["unknown".into()],
        vec!["x' OR 1=1".into()],
        vec!["name".into(); 33],
    ] {
        let mut view = definition();
        view.columns = columns;
        assert!(validate_view(&fields, &view).is_err());
    }
    let mut view = definition();
    view.query = "あ".repeat(129);
    assert!(validate_view(&fields, &view).is_err());
    let mut view = definition();
    view.filter.as_mut().unwrap().value = json!(0);
    assert!(validate_view(&fields, &view).is_err());
}

#[test]
fn view_shape_rejects_unknown_properties_and_requires_columns() {
    for input in [
        json!({"name":"x"}),
        json!({"name":"x","columns":["name"],"sql":"DROP"}),
        json!({"name":"x","columns":null}),
    ] {
        assert!(serde_json::from_value::<ViewDefinition>(input).is_err());
    }
    let view: ViewDefinition =
        serde_json::from_value(json!({"name":"x","columns":["name"]})).unwrap();
    assert_eq!(view.query, "");
    assert!(view.filter.is_none());
    validate_view(&schema().fields, &view).unwrap();
}
