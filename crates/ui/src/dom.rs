// SPDX-License-Identifier: Apache-2.0
use business_apps_domain::FieldKind;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Document, Element, HtmlButtonElement, HtmlElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement,
};

pub type Result<T = ()> = std::result::Result<T, JsValue>;

pub fn document() -> Document {
    web_sys::window()
        .expect("browser window")
        .document()
        .expect("browser document")
}

pub fn id(name: &str) -> Element {
    document()
        .get_element_by_id(name)
        .expect("Rust-generated UI element")
}

pub fn query(parent: &Element, selector: &str) -> Element {
    parent
        .query_selector(selector)
        .expect("static CSS selector")
        .expect("Rust-generated child")
}

pub fn all(parent: &Element, selector: &str) -> Vec<Element> {
    let nodes = parent
        .query_selector_all(selector)
        .expect("static CSS selector");
    (0..nodes.length())
        .filter_map(|i| nodes.item(i))
        .filter_map(|node| node.dyn_into().ok())
        .collect()
}

pub fn children(parent: &Element) -> Vec<Element> {
    let nodes = parent.children();
    (0..nodes.length()).filter_map(|i| nodes.item(i)).collect()
}

pub fn element(tag: &str, text: Option<&str>, class: Option<&str>) -> Result<Element> {
    let node = document().create_element(tag)?;
    node.set_text_content(text);
    if let Some(class) = class {
        node.set_class_name(class);
    }
    Ok(node)
}

pub fn append(parent: &Element, child: &Element) -> Result {
    parent.append_child(child)?;
    Ok(())
}

pub fn child(
    parent: &Element,
    tag: &str,
    attrs: &[(&str, &str)],
    text: Option<&str>,
) -> Result<Element> {
    let node = element(tag, text, None)?;
    for (name, value) in attrs {
        node.set_attribute(name, value)?;
    }
    append(parent, &node)?;
    Ok(node)
}

pub fn clear(parent: &Element) {
    parent.set_text_content(None);
}
pub fn text(name: &str, message: &str) {
    id(name).set_text_content(Some(message));
}
pub fn hidden(node: &Element, value: bool) {
    node.unchecked_ref::<HtmlElement>().set_hidden(value);
}
pub fn focus(node: &Element) {
    let _ = node.unchecked_ref::<HtmlElement>().focus();
}

pub fn disabled(node: &Element, value: bool) {
    if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
        node.set_disabled(value);
    } else if let Some(node) = node.dyn_ref::<HtmlSelectElement>() {
        node.set_disabled(value);
    } else if let Some(node) = node.dyn_ref::<HtmlTextAreaElement>() {
        node.set_disabled(value);
    } else if let Some(node) = node.dyn_ref::<HtmlButtonElement>() {
        node.set_disabled(value);
    }
}

pub fn value(node: &Element) -> String {
    if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
        node.value()
    } else if let Some(node) = node.dyn_ref::<HtmlSelectElement>() {
        node.value()
    } else if let Some(node) = node.dyn_ref::<HtmlTextAreaElement>() {
        node.value()
    } else {
        String::new()
    }
}

pub fn set_value(node: &Element, value: &str) {
    if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
        node.set_value(value);
    } else if let Some(node) = node.dyn_ref::<HtmlSelectElement>() {
        node.set_value(value);
    } else if let Some(node) = node.dyn_ref::<HtmlTextAreaElement>() {
        node.set_value(value);
    }
}

pub fn checked(node: &Element) -> bool {
    node.unchecked_ref::<HtmlInputElement>().checked()
}
pub fn set_checked(node: &Element, value: bool) {
    node.unchecked_ref::<HtmlInputElement>().set_checked(value);
}

pub fn required(node: &Element, value: bool) {
    if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
        node.set_required(value);
    } else if let Some(node) = node.dyn_ref::<HtmlSelectElement>() {
        node.set_required(value);
    } else if let Some(node) = node.dyn_ref::<HtmlTextAreaElement>() {
        node.set_required(value);
    }
}

pub fn kind_name(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Textarea => "textarea",
        FieldKind::Number => "number",
        FieldKind::Boolean => "boolean",
        FieldKind::Date => "date",
        FieldKind::Select => "select",
    }
}

pub fn kind(value: &str) -> FieldKind {
    match value {
        "textarea" => FieldKind::Textarea,
        "number" => FieldKind::Number,
        "boolean" => FieldKind::Boolean,
        "date" => FieldKind::Date,
        "select" => FieldKind::Select,
        _ => FieldKind::Text,
    }
}
