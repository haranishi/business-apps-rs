# Check native Rust, WASM, and browser behavior

Use Rust stable (minimum 1.88), a C toolchain, and the WASM target/CLI from [Rust UI setup](rust-ui.md).
Python 3.10+ supports docs/benchmarks; Node.js 22+ supports Playwright browser tests.
The app build uses Rust tooling; Node.js is not part of its runtime or WASM build.

## Run the checks that cover the change

```sh
scripts/check.sh
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check advisories licenses bans sources
npm ci
npm audit --audit-level=high
npx playwright install chromium
npm run test:e2e
```

`scripts/check.sh` builds the release browser assets, checks formatting, runs native Clippy/tests, runs WASM Clippy, and checks documentation.
For a documentation-only change, `python3 scripts/check_docs.py` checks local links and document constraints.
Run affected checks during development, then record the complete checks required for the final change.
Generated assets must be rebuilt before compiling the server; [build commands](rust-ui.md) preserve that order.

## Exercise operations and saved data

Rust tests cover isolation, validation, stale updates/deletes, pagination/search, body limits, schema edits, database upgrades, and reopening.
WASM checks compile the browser target; a successful compilation does not establish correct interaction behavior.
Browser regression tests cover the existing operations while application layout/events run in Rust WASM.
Use create → save → search → edit → reopen, required-error recovery, competing revisions, keyboard focus, and pending locks.
Include old-form edits after a schema addition and delayed responses while a newer draft is open.
Named-view tests also cover conditions/column order, hidden-field retention, full-schema CSV, revisions, retries and delayed catalog/record responses.

Playwright starts and stops a separate loopback server with synthetic data and an isolated temporary database.
It does not attach to your daily workspace database. Projects cover 390, 768, and 1440 px widths.
Screenshots, traces, and reports go to Git-ignored `.artifacts/`.
Use [UI diagrams](ui.md) for the expected flow and the [project harness](../HARNESS.md) for independent review.

## Separate CI configuration from a passing run

CI covers native Rust, WASM, the minimum supported Rust version, documentation, dependency policy, and browser flows.
Server/domain PRs also run a release HTTP baseline (10,000 records, concurrency 8, 200 samples per workload), retained for seven days.
Its latency values describe that shared runner, not a stable performance threshold or a comparison with other machines.
Record the actual run URL, source SHA, platform/toolchain, results, and any skipped or unverified checks.
A workflow file describes intended checks; it is not evidence that a particular revision passed.
Every completed browser job retains synthetic reports/screenshots for seven days; failures also retain traces.
CI checks Rust advisories and high/critical npm advisories, alongside dependency licenses and sources.
The weekly run detects newly published advisories even when no source changes are pushed.
PRs run once per update; pushes to main verify the merged result.
Branch protection and maintainer merge decisions are separate from CI; the checks do not publish or deploy the app.

## Measure speed with a release build

```sh
cargo xtask build
python3 scripts/benchmark.py --records 10000 --requests 500 --concurrency 8 --output .artifacts/benchmark.json
```

The benchmark creates and removes its own synthetic SQLite database and server process.
It measures end-to-end HTTP latency for indexed record reads, first-page listing, text/typed scans, and writes.
Record build mode, machine, dataset size, concurrency, sample count, p50/p95/p99, and throughput.
The Python client contributes overhead; HTTP results do not measure WASM download, browser rendering, or interaction latency.
Compare the same workloads and conditions before claiming an improvement or a product comparison.
Search scans per-app string values. SQLite WAL uses durable FULL synchronization during the benchmark.
See [the measured baseline](performance.md) for recorded conditions and limits.
