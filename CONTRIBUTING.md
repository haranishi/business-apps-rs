# Your first contribution

You can help through bug reproduction, documentation, UI translations, tests, or focused code fixes.
Read [the short design](docs/design.md) first; the repository is an early local preview.

## Run the project

Install the Rust toolchain named in `rust-toolchain.toml`, then:

```sh
cargo run -p business-apps-server
```

Open `http://127.0.0.1:8080`. Use synthetic records; the preview has no team authentication.
The default SQLite file is `.data/business-apps.sqlite` and is ignored by Git.
To test with an isolated database: `BIZAPP_DB=.data/test.sqlite BIZAPP_BIND=127.0.0.1:8081 cargo run -p business-apps-server`.

## Find the right file

| Change | Start here | Check |
| --- | --- | --- |
| Field rules | `crates/domain/src/lib.rs` | `cargo test -p business-apps-domain` |
| Saved data | `crates/server/src/store.rs` | Storage/reopen integration tests |
| HTTP behavior | `crates/server/src/lib.rs` | API integration tests |
| Browser behavior | `web/app.js`, `web/style.css` | `npm run test:e2e` |
| Design or diagrams | `docs/` | `python3 scripts/check_docs.py` |

## Verify a change

```sh
scripts/check.sh
npm ci
npx playwright install chromium
npm run test:e2e
cargo install cargo-deny --version 0.20.2 --locked
cargo deny check licenses bans sources
```

Browser tests start a separate process with a temporary SQLite database.
Use [the harness](HARNESS.md) for acceptance evidence and [testing notes](docs/testing.md) for CI details.

## Propose and review

Open an Issue first for a new API, breaking saved-data change, major dependency, or team/network feature.
For small fixes, a focused PR with a reproducer and validation is welcome immediately.
PR titles and descriptions use English; discussion and contributions may use English or Japanese.
Explain the problem, result, tests, and any compatibility risk. Update docs with changed contracts.
Submit original work or properly attributed compatible code under Apache-2.0.
Maintainers decide acceptance and merge; CI verifies reproducible checks.
Use [private vulnerability reporting](SECURITY.md) for a security issue.

## Terms

An **app** is a named schema; a **field** describes a value; a **record** is one entry.
A **revision** changes after each save and prevents stale edits from replacing newer data.
A **migration** upgrades the persisted database format with a documented compatibility path.
