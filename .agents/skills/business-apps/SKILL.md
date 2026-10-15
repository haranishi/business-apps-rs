---
name: business-apps
description: Implement or review this repository's Rust business-app schemas, SQLite persistence, HTTP API, and browser workflows with the existing integrity and performance contracts.
---

# Develop business-apps-rs

Read [design](../../../docs/design.md) and [the harness](../../../HARNESS.md).
For HTTP changes read [API v1](../../../docs/api.md); for screens read [UI stories](../../../docs/ui.md).

## Preserve the boundaries

- Keep field/type validation in `crates/domain`; do not add HTTP or database dependencies there.
- Submit database work through `Store::call`; the connection stays on its dedicated worker.
- Scope all record SQL to `app_id` and compare revisions during update/delete.
- Schema changes require a migration plus an old-database reopen test.
- Compare app revisions on schema updates; keep existing keys/types/required/options and allow only optional additions.
- Keep the local preview's loopback, Host, Origin, and mutation-header protections.
- Render records and field labels as text; retain drafts when a save fails.
- Write browser layout/behavior in `crates/ui`, using shared domain types and bounded event listeners.
- Generate assets with `cargo xtask build-web`; keep generated JS/WASM outside Git.

## Choose evidence by changed behavior

Use domain tests for validation; API/storage tests for isolation, conflicts, and persistence.
Use browser tests for the full user flow, including error recovery and focus.
For speed changes, run a release build and the benchmark; include workload, samples, p50/p95/p99, and machine.
Search is a per-app scan of string values; do not describe it as indexed full-text search.

## Prepare a contribution

Run `scripts/check.sh` (native and WASM), relevant browser tests, and dependency license checks including wasm32.
Update the contract document/diagram with public behavior changes.
Keep a PR focused on one outcome and explain remaining limitations.
This skill does not authorize pushes, merges, releases, or changes outside the repository.
