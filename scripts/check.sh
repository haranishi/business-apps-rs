#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo xtask build-web
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p business-apps-ui --target wasm32-unknown-unknown --locked -- -D warnings
cargo test --workspace --locked
python3 scripts/check_docs.py
