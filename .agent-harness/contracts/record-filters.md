# Typed exact-match record filters

[Issue 11](https://github.com/haranishi/business-apps-rs/issues/11) records the API/UX proposal before implementation.
An operator needs to find records by status, checkbox, quantity, date, or missing values without writing code.

## Decisions and boundaries

- Add one domain-validated exact-value filter, combined with literal text search using AND.
- Share bound SQL for listing and snapshot CSV; retain app isolation, cursor semantics and export completeness limits.
- Null means missing/null/empty text; preserve zero, false, whitespace and numeric equality explicitly.
- Keep existing Store methods as unfiltered wrappers; preserve API behavior when the optional filter is absent.
- No new dependency, migration, saved view, range/OR query, indexed-query claim, permissions or deployment.

## UX layers

| Layer | Decision |
| --- | --- |
| Strategy | Find work using the meaning of an app's fields |
| Requirements | Typed exact/empty condition, explicit apply/clear, honest result scope |
| Structure | Optional disclosure within current search, shared with list/CSV |
| Skeleton | Field/mode/value controls, apply/clear, visible applied-condition summary |
| Surface | Japanese labels, native typed controls, keyboard access and responsive wrapping |

## Acceptance and verification

- Cover all six field types, null/missing/empty/whitespace, zero/false and invalid input.
- Verify app scoping, literal keys/values, query composition, descending cursors and CSV parity.
- Keep drafts distinct from applied conditions, discard superseded requests, and retry errors without exporting an older view.
- Preserve CRUD, revisions, pending locks, schema editing, saved values, reopening and CSV focus regression.
- Run native Linux/macOS, Rust 1.88, WASM Clippy, dependency/advisory checks and Chromium at 390/768/1440 px.
- Inspect screenshots and receive independent read-only standards/specification review.
- Use at most three improvement rounds; stop after the same failure twice.
