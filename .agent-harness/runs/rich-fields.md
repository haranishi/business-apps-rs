# Rich-field validation

Runtime/UI revision: `9e5c611`; design [Issue 2](https://github.com/haranishi/business-apps-rs/issues/2), [PR 3](https://github.com/haranishi/business-apps-rs/pull/3).

## Verified behavior

- `scripts/check.sh`: format/Clippy, 18 Rust cases, documentation links, and JavaScript syntax passed.
- Rust 1.88 workspace/all-targets and dependency license/bans/source checks passed; no dependencies added.
- Full Chromium suite: 21 cases passed at 390/768/1440 px.
- API rejects invalid dates and unknown choices without saving; rich values survive reopening.
- A format-1 fixture retains exact field/value JSON, IDs, and record revisions after advancing to format 2.
- Independent code review found Unicode option truncation and untouched textarea line-ending changes.
- Both were reproduced, fixed, regression-tested, and independently rechecked with no new findings.
- The browser now accepts all 50 × 80-character options, including supplementary Unicode characters.
- Untouched API-origin CRLF/CR text stays byte-for-byte unchanged when another field is edited.
- Independent UI-only operation at all three widths covered duplicate-choice recovery, required drafts, save/search/reopen, keyboard cancel/focus, and pending-write locks; no page errors.
- Screenshots at all three widths were inspected; native select keyboard behavior in headless macOS also reproduces in a minimal standard form.

## CI correction

The first push run exposed same-timestamp temporary database collisions on macOS.
A process-local atomic sequence now makes those test paths unique.
The next runtime head is checked by [PR CI](https://github.com/haranishi/business-apps-rs/pull/3/checks); earlier successes are not reused for later commits.

## Measured performance

[Raw run and conditions](../../docs/benchmarks/rich-fields.json): release, Rust 1.99, arm64 macOS, 14 logical CPUs, SSD project database, SQLite WAL/FULL.
10,000 initial records, concurrency 8, 500 samples per workload, localhost HTTP including Python overhead.
p95: read 1.045 ms, page of 30 records 1.236 ms, no-match text scan 30.304 ms, write 2.037 ms.
This does not measure rich-field-specific workloads or establish superiority to another product.

Schemas are still immutable, and this remains a loopback-only preview without team authentication.
The stopped-database-copy/downgrade limitation is documented in [field rules](../../docs/fields.md).
