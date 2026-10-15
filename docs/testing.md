# Test environments and evidence

Use Rust stable (minimum supported version: 1.88), Node.js 22+, and Python 3.10+ on Linux/macOS.
Running the app itself needs only Rust and a browser; Node is used for development checks and browser tests.

## Local checks

```sh
scripts/check.sh
cargo deny check licenses bans sources
npm ci
npx playwright install chromium
npm run test:e2e
```

Rust tests exercise app isolation, type errors, stale updates/deletes, concurrent edits, pagination, literal search, body limits, date/choice validation, version-1 upgrades, and database reopening.
Browser tests use synthetic data, an isolated temporary database, and a separate loopback port.
Playwright starts and stops its own server; it does not attach to your daily workspace database.
Three projects cover 390, 768, and 1440 px widths.
Screenshots, traces, and reports go to `.artifacts/` and are ignored by Git.

## GitHub Actions

`CI` runs formatting/Clippy/tests on Linux and macOS, the minimum Rust version on Linux, documentation checks, dependency license/source checks, and Chromium flows on Linux.
Actions are pinned to commit SHAs. PR jobs have read-only repository permissions and use no deployment secrets.
Failed browser tests upload their synthetic traces/screenshots for inspection.
This workflow does not merge PRs, deploy a service, or publish packages.
A workflow file is not evidence of a successful run; record the run URL and exact head SHA.
Branch protection is separate from CI and must be configured by a maintainer if desired.

## Measure speed

```sh
cargo build --release --locked -p business-apps-server
python3 scripts/benchmark.py --records 10000 --requests 500 --concurrency 8 --output .artifacts/benchmark.json
```

The benchmark creates and removes its own synthetic SQLite database and server process.
It records end-to-end HTTP latency for indexed record reads, first-page listing, no-match text scans, and writes.
Results include build mode, machine, dataset size, concurrency, samples, p50/p95/p99, and throughput.
Python client overhead and a single machine limit interpretation; it does not establish performance against another product.
Text search scans per-app string values; compare realistic datasets before introducing an index or different storage backend.
SQLite uses WAL with durable FULL synchronization; the benchmark does not disable durability.
