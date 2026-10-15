// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::{App, NewApp, validate_app_update, validate_values};
use serde_json::json;

fn current() -> App {
    serde_json::from_value(json!({"id":1,"revision":3,"name":"Tasks","fields":[
        {"key":"name","label":"Name","kind":"text","required":true},
        {"key":"status","label":"Status","kind":"select","options":["Open","Done"]}
    ]}))
    .unwrap()
}
fn input(app: &App) -> NewApp {
    NewApp {
        name: app.name.clone(),
        fields: app.fields.clone(),
    }
}

#[test]
fn labels_order_and_optional_additions_preserve_old_values() {
    let app = current();
    let mut updated = input(&app);
    updated.name = "Renamed tasks".into();
    updated.fields[0].label = "Title".into();
    updated.fields.reverse();
    updated
        .fields
        .push(serde_json::from_value(json!({"key":"due","label":"Due","kind":"date"})).unwrap());
    validate_app_update(&app, &updated).unwrap();
    validate_values(
        &updated.fields,
        json!({"name":"Existing","status":"Open"})
            .as_object()
            .unwrap(),
    )
    .unwrap();
}

#[test]
fn destructive_or_retroactive_changes_are_rejected() {
    let app = current();
    let mut variants = Vec::new();
    let mut change = input(&app);
    change.fields.remove(0);
    variants.push(change);
    let mut change = input(&app);
    change.fields[0].key = "renamed".into();
    variants.push(change);
    let mut change = input(&app);
    change.fields[0].kind = business_apps_domain::FieldKind::Number;
    variants.push(change);
    let mut change = input(&app);
    change.fields[0].required = false;
    variants.push(change);
    let mut change = input(&app);
    change.fields[1].required = true;
    variants.push(change);
    let mut change = input(&app);
    change.fields[1].options.push("New".into());
    variants.push(change);
    let mut change = input(&app);
    change.fields[1].options.reverse();
    variants.push(change);
    let mut change = input(&app);
    change.fields.push(
        serde_json::from_value(json!({"key":"due","label":"Due","kind":"date","required":true}))
            .unwrap(),
    );
    variants.push(change);
    let mut change = input(&app);
    change.fields.push(change.fields[0].clone());
    variants.push(change);
    let mut change = input(&app);
    change.name = " ".into();
    variants.push(change);
    for change in variants {
        assert!(validate_app_update(&app, &change).is_err(), "{change:?}");
    }
}
