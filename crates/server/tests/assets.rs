// SPDX-License-Identifier: Apache-2.0
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use business_apps_server::{router, store::Store};
use tower::ServiceExt;

#[tokio::test]
async fn embedded_rust_ui_is_served_with_correct_mime_and_scoped_csp() {
    let app = router(Store::open(":memory:").unwrap());
    for (path, mime) in [
        ("/", "text/html; charset=utf-8"),
        ("/assets/start.js", "text/javascript; charset=utf-8"),
        (
            "/assets/business_apps_ui.js",
            "text/javascript; charset=utf-8",
        ),
        ("/assets/business_apps_ui_bg.wasm", "application/wasm"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("host", "127.0.0.1:8080")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()["content-type"], mime);
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap();
        assert!(policy.contains("'wasm-unsafe-eval'"));
        assert!(!policy.contains("'unsafe-eval'") && !policy.contains("'unsafe-inline'"));
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        if path.ends_with(".wasm") {
            assert!(body.starts_with(b"\0asm\x01\0\0\0"));
        } else if path == "/" {
            let html = std::str::from_utf8(&body).unwrap();
            assert!(html.contains("/assets/start.js"));
            assert!(html.contains("boot-status"));
        } else if path.ends_with("start.js") {
            assert!(
                std::str::from_utf8(&body)
                    .unwrap()
                    .contains("./business_apps_ui.js")
            );
        }
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/business_apps_ui_bg.wasm")
                .header("host", "attacker.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
