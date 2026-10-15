# Schema-editing validation

API revision: `8c1ba55`; reviewed browser/runtime revision: `c5d03cc`.
Design [Issue 4](https://github.com/haranishi/business-apps-rs/issues/4), [PR 5](https://github.com/haranishi/business-apps-rs/pull/5).

## Evidence

- `scripts/check.sh`: formatting/Clippy, 24 Rust cases, documentation links/SVGs, and JavaScript syntax passed.
- Rust 1.88 all-targets and dependency license/bans/source checks passed; no dependencies added.
- Full Chromium suite: 36 cases passed at 390/768/1440 px, including the final review fixes.
- Domain mutation matrix rejects destructive/retroactive changes; optional additions validate old values.
- API/storage checks cover stale app revisions, exactly-one-winner concurrency, app isolation, rejected changes, exact record JSON/revision preservation, and reopen.
- Owned format-1/2 fixtures advance to 3; existing schema/value JSON and record revisions stay intact.
- Failed migration leaves prior metadata and rows intact; unrelated/future formats remain refused.
- The first browser run exposed Escape losing the settings invoker focus; re-enabling/focusing before opening the dialog fixed it.
- Independent review reproduced three additional issues: old-form tabs dropping new values, delayed settings loads replacing newer drafts, and JavaScript trimming immutable API-origin choice labels.
- Each was fixed and regression-tested; independent temp-DB/API/Chromium rechecks found no new issues.
- Independent UI-only operation at all widths confirmed save/reload, keyboard movement/focus, conflicts, pending locks, old-tab value retention, and delayed-load draft retention; page errors were zero.
- UI review: 19/20; affected experience: 20/20. Screen-reader context remains unverified.
- Updated synthetic screenshots show the table, six-type builder, editor, and form settings; no real business data is included.

The final documentation commit is checked separately by [PR CI](https://github.com/haranishi/business-apps-rs/pull/5/checks); runtime successes are not treated as checks of later commits.

## Performance

[Raw run and conditions](../../docs/benchmarks/schema-editing.json): release, Rust 1.99, arm64 macOS, 14 logical CPUs, SSD project database, SQLite WAL/FULL.
10,000 initial records, concurrency 8, 500 samples per workload, localhost HTTP including Python overhead.
p95: read 1.230 ms, page of 30 records 1.420 ms, no-match text scan 26.455 ms, write 2.096 ms.
This is a repeatable local check, not a benchmark of schema-edit throughput or a comparison with another product.

## Remaining scope

Older tabs display their loaded schema until refreshed, while preserving all loaded record values on save.
Only optional additions and presentation edits are supported; destructive schema changes need a separate migration design.
This remains loopback-only without team authentication, authorization, or production deployment support.
