# business-apps-rs

[日本語](README.ja.md) · [Contribute](CONTRIBUTING.md) · [Design a change](docs/development.md) · [Roadmap](docs/roadmap.md)

Build customer lists, inventory apps, and task trackers from typed fields.
Rust implements the browser layout and interactions, API, domain rules, and persistence.
The browser UI compiles to WebAssembly; CSS and generated JavaScript glue are also delivered.

**Status: local development preview.** Team collaboration and supported deployment are future stages.
This is an independent project, unaffiliated with Cybozu; kintone API compatibility is not implemented.

![Task app with app navigation, search, form settings, and saved records](docs/assets/preview.png)

The screenshot uses synthetic data. See [screen diagrams and the editor](docs/ui.md).

## What you can build today

| Area | Local preview |
| --- | --- |
| Fields | Text, multiline text, number, checkbox, date, and single choice |
| Records | Create/read/edit/delete, text search and [typed field filters](docs/record-filters.md); durable SQLite and revision checks |
| Export | [Spreadsheet-oriented CSV](docs/csv.md) of all matching pages, with formula protection and explicit size limits |
| Form settings | Rename app/labels, reorder fields, add optional fields; existing types/required flags/choices stay fixed |
| Named views | [Save applied conditions and visible-column order](docs/saved-views.md), recall, update, or delete the list definition |
| Planned | Team login, role-based permissions, audit history, import, workflows, packaged deployment |

The [roadmap](docs/roadmap.md) separates these stages. Text search scans string values within one app.

## Create your first customer list

Install Rust stable (minimum 1.88) and a C toolchain: Xcode Command Line Tools on macOS or GCC/Clang on Linux.
The target and CLI installation below are one-time setup; Node.js is only needed for browser tests.

```sh
git clone https://github.com/haranishi/business-apps-rs.git
cd business-apps-rs
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cargo xtask dev
```

1. Open `http://127.0.0.1:8080` and choose **アプリを作る**; name it `顧客管理`.
2. Set two text fields: required `顧客名` and optional `連絡先`; choose **アプリを作成**.
3. Choose **レコードを追加**, enter `架空商事` and `hello@example.invalid`, then **保存**.
4. Search for `架空商事`, open **編集**, and save a change.
5. Open **フォーム設定** to rename labels, reorder fields, or add an optional field.
6. Apply a search/condition, then **この条件で一覧を保存** to name a reusable list and choose its columns.

Restart the server to reopen saved records. The database is `.data/business-apps.sqlite`, ignored by Git.
The preview accepts loopback connections only and has no authentication; use synthetic data.
Existing fields cannot be removed; their keys, types, required flags, and choices stay fixed to preserve saved values.
Existing databases upgrade to format 4; read the [backup and rollback steps](docs/saved-views.md#stored-data-upgrade-and-recovery) before upgrading.

## Build the server and browser UI

| Command | Result |
| --- | --- |
| `cargo xtask dev` | Build release WASM assets, then build and run the debug server |
| `cargo xtask build` | Build release WASM assets and the release server |
| `cargo xtask build-web` | Regenerate browser assets without building the server |

The server embeds the assets into one binary. After generating them, `cargo run -p business-apps-server` also works.
[The Rust UI guide](docs/rust-ui.md) explains `crates/ui`, generated `web/pkg`, CSS, and rebuilds.
Python 3.10+ supports documentation and benchmark tooling; Node.js 22+ supports Playwright tests.

## Understand the design or make a contribution

| Goal | Start here |
| --- | --- |
| Design a feature and its acceptance checks | [Development steps](docs/development.md) |
| Understand runtime and data boundaries | [Architecture](docs/design.md) and [API v1](docs/api.md) |
| Understand screens and interactions | [UI stories and diagrams](docs/ui.md) |
| Make a first contribution | [Contribution guide](CONTRIBUTING.md) |
| Run native, WASM, and browser checks | [Testing](docs/testing.md) and [project harness](HARNESS.md) |
| Evaluate speed | [Workloads and measured baseline](docs/performance.md) |

Bug reproduction, documentation, translation, accessibility, tests, and focused fixes are welcome.
Discuss new APIs, migrations, major dependencies, or team/network features in an Issue first.
Performance claims need workload, build mode, machine, sample count, and measurements; Rust alone proves no speed advantage.
CI configuration describes checks; each run and source revision provide the validation evidence.

## License and data

Original code, docs, diagrams, and screenshots use [Apache-2.0](LICENSE); dependencies retain their own licenses.
See [NOTICE](NOTICE), [licensing and provenance](docs/licenses.md), and [private security reporting](SECURITY.md).
Keep business databases, credentials, and local settings out of contributions; use synthetic examples.
kintone is a registered trademark of Cybozu, Inc.; this project has no affiliation or endorsement.
