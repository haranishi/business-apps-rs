# Your first contribution

Bug reproduction, documentation, UI translations, tests, and focused fixes are welcome.
Start with [one working app](README.md#create-your-first-customer-list), then read [how to design a change](docs/development.md).
There is no contributor registration requirement. Discussion and contributions may use English or Japanese.

## Prepare a working checkout

Use Rust stable (minimum 1.88), a C toolchain, and the WASM target/CLI described in [Rust UI setup](docs/rust-ui.md).
From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cargo xtask dev
```

Open `http://127.0.0.1:8080`. Use synthetic records; this preview has no team authentication.
The default SQLite file is `.data/business-apps.sqlite`, ignored by Git.
For a separate manual workspace: `BIZAPP_DB=.data/test.sqlite BIZAPP_BIND=127.0.0.1:8081 cargo xtask dev`.
Python 3.10+ is used by docs/benchmark tooling; Node.js 22+ is used by browser tests.

## Choose one contribution and its checks

| Change | Start here | Relevant check |
| --- | --- | --- |
| Explain a confusing step or diagram | `README.md`, `docs/` | `python3 scripts/check_docs.py` |
| Field rules or validation | `crates/domain/` | `cargo test -p business-apps-domain` |
| Saved data or HTTP behavior | `crates/server/` | `cargo test -p business-apps-server` and reopen/API cases |
| Browser layout, state, events, or labels | `crates/ui/` | WASM checks and affected browser flows |
| Colors, spacing, or responsive rules | `web/style.css` | Browser flows and screenshots at three widths |
| WASM generation and embedding | [Rust UI build guide](docs/rust-ui.md) | `cargo xtask build` and browser loading |

Generated `web/pkg` files are ignored by Git; change Rust source and regenerate them.
A first issue can include the version, environment, smallest synthetic input, exact actions, and expected/actual result.
Do not create a new extension system or public API to solve a small isolated fix without discussing the scope first.

## Verify before proposing the change

Use the checks that cover the changed behavior; the [testing guide](docs/testing.md) explains the full check graph.
Rust/UI changes use the following checks after setup:

```sh
scripts/check.sh
npm ci
npx playwright install chromium
npm run test:e2e
cargo install cargo-deny --version 0.20.2 --locked
cargo deny check licenses bans sources
```

`scripts/check.sh` builds browser assets before native and WASM checks.
Browser tests start an isolated server/database and retain synthetic failure evidence in `.artifacts/`.
For documentation-only work, start with the documentation check in the table above.
Record the commands, source revision, outcomes, and anything you could not verify.

## Propose a focused PR

Open an Issue first for a new API, breaking persisted-data change, major dependency, or team/network feature.
Small fixes can be proposed directly with a reproducer and validation.
PR titles and descriptions use English. Explain the user-visible result, compatibility risks, and checks.
Follow the [development steps](docs/development.md) and [project harness](HARNESS.md); update contracts and diagrams when behavior changes.
Submit original work or properly attributed compatible work under Apache-2.0; see [provenance](docs/licenses.md).
Keep databases, credentials, local settings, generated assets, and test artifacts out of Git.
Maintainers decide acceptance and merge; CI verifies checks for a particular revision.
Use [private vulnerability reporting](SECURITY.md) for a security issue.

## Shared terms

An **app** is a named schema; a **field** describes a value; a **record** is one entry.
Apps and records have separate **revisions**, compared on their respective saves to reject stale updates.
A stable **field key** identifies stored values even when a label or position changes.
A **migration** upgrades persisted data with an explicit compatibility and recovery path.
