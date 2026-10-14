# business-apps-rs

An independent, Rust-based open-source foundation for creating business apps from typed fields.

This project is being built in small, reviewable increments. The first milestone is a local app builder with persistent records, a browser UI, and reproducible tests.

Licensed under [Apache-2.0](LICENSE). This project is independent of Cybozu and kintone; kintone is a registered trademark of Cybozu, Inc.

## Follow the implementation

The first working app builder is in [Draft PR 1](https://github.com/haranishi/business-apps-rs/pull/1).
Read the [full README](https://github.com/haranishi/business-apps-rs/blob/feat/local-app-builder/README.md), [日本語README](https://github.com/haranishi/business-apps-rs/blob/feat/local-app-builder/README.ja.md), [design and diagrams](https://github.com/haranishi/business-apps-rs/blob/feat/local-app-builder/docs/design.md), and [contribution guide](https://github.com/haranishi/business-apps-rs/blob/feat/local-app-builder/CONTRIBUTING.md).

To try the current implementation before it is merged:

```sh
git clone --branch feat/local-app-builder https://github.com/haranishi/business-apps-rs.git
cd business-apps-rs
cargo run -p business-apps-server
```

Rust stable and a C compiler are required. Open `http://127.0.0.1:8080` and use synthetic data; this preview has no user/permission model.
