// SPDX-License-Identifier: Apache-2.0
use std::{env, fs, path::PathBuf};
fn main() {
    let package = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo package path"));
    let assets = package.join("../../web/pkg");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    for file in [
        "business_apps_ui.js",
        "business_apps_ui_bg.wasm",
        "start.js",
    ] {
        let source = assets.join(file);
        println!("cargo:rerun-if-changed={}", source.display());
        fs::copy(&source, output.join(file)).unwrap_or_else(|error| {
            panic!("Rust UI assets are missing or unreadable ({file}: {error}). Run cargo xtask build-web first. See docs/rust-ui.md.")
        });
    }
}
