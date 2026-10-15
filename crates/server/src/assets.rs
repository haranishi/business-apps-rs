// SPDX-License-Identifier: Apache-2.0
//! Rust-generated UI assets embedded during the explicit xtask build.
use axum::response::{Html, IntoResponse};
const INDEX: &str = r#"<!doctype html>
<html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>business-apps-rs | 業務アプリ</title><link rel="stylesheet" href="/style.css">
<script type="module" src="/assets/start.js"></script></head>
<body><p id="boot-status" role="status">画面を起動しています…</p><noscript>この画面にはJavaScriptとWebAssemblyに対応したブラウザが必要です。</noscript></body></html>"#;
pub(crate) async fn index() -> Html<&'static str> {
    Html(INDEX)
}
pub(crate) async fn glue() -> impl IntoResponse {
    (
        [("content-type", "text/javascript; charset=utf-8")],
        include_str!(concat!(env!("OUT_DIR"), "/business_apps_ui.js")),
    )
}
pub(crate) async fn start() -> impl IntoResponse {
    (
        [("content-type", "text/javascript; charset=utf-8")],
        include_str!(concat!(env!("OUT_DIR"), "/start.js")),
    )
}
pub(crate) async fn wasm() -> impl IntoResponse {
    (
        [("content-type", "application/wasm")],
        include_bytes!(concat!(env!("OUT_DIR"), "/business_apps_ui_bg.wasm")).as_slice(),
    )
}
