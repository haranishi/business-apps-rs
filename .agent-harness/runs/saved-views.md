# Named-view verification

[Issue 13](https://github.com/haranishi/business-apps-rs/issues/13) and [Draft PR 14](https://github.com/haranishi/business-apps-rs/pull/14) cover reusable applied conditions and visible-column order.
The [contract](../contracts/saved-views.md) limits this work to local app-scoped definitions; records, editors and CSV retain all fields.

## Round 1: domain, API and persistence

- Core head `64c78dd` added validation, scoped CRUD, revisions, cap/name transactions and atomic format-4 migration.
- Independent specification and standards source reviews found no demonstrated core defect.
- [Initial CI](https://github.com/haranishi/business-apps-rs/actions/runs/38741328791) exposed a test expecting 400 for an Axum JSON-shape rejection of 422.
- The expectation now distinguishes shape rejection from domain 400 and asserts no rejected view was persisted.
- Added the reviewer's suggested regression: an early-format ALTER succeeds, later table creation fails, and the entire original schema/data/version remain intact.

## Round 2: UI and full checks

- Head `b302ea7` passed [all five required CI checks](https://github.com/haranishi/business-apps-rs/actions/runs/38743395445).
- `scripts/check.sh`: native/WASM Clippy, formatting, 56 Rust cases and documentation checks passed on Linux and macOS.
- Rust 1.88 all-targets/WASM checks, dependency advisories/licenses/bans/sources and high/critical npm audit passed.
- 114 Chromium cases passed across 390/768/1440 px, including actual WASM startup and view operations.
- Flows cover applied conditions versus drafts, column order, hidden-field retention, full-schema/all-page CSV, persistence, conflicts, explicit reload, retry, pending locks and focus.
- The [release baseline](https://github.com/haranishi/business-apps-rs/actions/runs/38743395006) also passed for this head's PR merge ref.

## Round 3: independent review and regression checks

Independent specification/standards reviews identified candidates for catalog responses overwriting newer work, obsolete app-selection continuations invalidating an editor, and native single-line input changing valid API-origin names.
Head `41fc8c6` added targeted browser regressions before implementation changes.
The [reproduction run](https://github.com/haranishi/business-apps-rs/actions/runs/38744106069) failed all four new cases at all three widths; 114 other cases passed.
Catalog refresh now updates only the catalog/status, retaining a deleted active snapshot with explicit reset/save-as-new options.
App selections have a separate monotonic generation, preventing obsolete same-ID continuations from invalidating an editor.
Settings saves preserve an unchanged API-origin name despite native single-line CR/LF normalization.
The reproduction regressions remain in the normal browser suite.
Final required CI, browser regressions and the release baseline must pass for the final PR head; earlier successes do not establish that result.

## Visual and security evidence limits

The main contributor and an independent visual reviewer inspected six actual round-2 screenshots at the three widths.
The reviewer found no confirmed defect in the added panel/dialog; eight visible categories scored 2/2 each.
This is not a full UI/experience harness pass: still images do not verify keyboard behavior, measured contrast or perceived speed.
The first captures accumulated prior synthetic apps and omitted the mobile dialog footer in full-page stitching.
Representative evidence now isolates the app catalog in the screenshot fixture and includes viewport/footer captures, alongside real record/view API operations.
Automated focus/pending/error paths are separate evidence from manual usability evaluation.

Independent static security/provenance review of `4416fd2...b302ea7` found no actionable regression.
Parameterized app-scoped SQL, write locks, revisions, bounded input, DOM text APIs and existing local-origin protections were inspected.
No dependencies, proprietary code/assets, credentials, live records or deployment settings were added.
Source inspection and dependency checks do not establish absence of vulnerabilities or legal clearance.
No manual local app session, screen-reader validation, isolated dynamic security audit, or long-term user study was performed.

## Reproduction environment

Heavy builds and browser runs use GitHub Actions with synthetic temporary SQLite databases.
Local checks use `cargo fmt --all -- --check`, `python3 scripts/check_docs.py`, `node --check tests/views.spec.js` and `git diff --check`.
Performance uses release Rust, Linux x86_64, 10,000 records, concurrency 8, 200 samples per workload and SQLite WAL/FULL.
Raw benchmark JSON records the exact PR merge revision and machine; values include Python HTTP overhead on shared hardware.
No stable latency threshold or cross-product performance claim is made.
The preview remains loopback-only; team authentication/authorization and network deployment require separate design.
The PR remains unmerged; no release or public deployment is part of this work.
