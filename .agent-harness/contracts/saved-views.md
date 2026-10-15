# Named record views

[Issue 13](https://github.com/haranishi/business-apps-rs/issues/13) records the public API and migration proposal before implementation.

An operator should recall a daily work list without rebuilding its conditions or scanning unrelated columns.
This change adds saved query/filter definitions and visible-column order to the existing local workspace.

## Scope and compatibility

- App-scoped view CRUD with revisions, unique trimmed names, 1–32 known column keys and a 50-view cap.
- Original Rust domain/API/storage/WASM implementation, using existing dependencies and local-origin protections.
- Atomic SQLite format 4 migration from formats 1/2/3; preserve all existing app/record values and revisions.
- Selecting a view applies stored conditions; record editors and CSV retain all schema fields.
- Saving captures applied conditions, preserves drafts on failure, and compares revisions for update/delete.
- View-list failures have their own retry and cannot block the existing record workflow.
- No auth, network deployment, release, record sorting, saved choice/bookmark or external integration.

## UX layers

| Layer | Decision |
| --- | --- |
| Strategy | Make recurring business lists quick to recall |
| Requirements | Named conditions, column selection/order, save/update/delete and recovery |
| Structure | Selector in the existing workspace; native settings dialog |
| Skeleton | Current list and modification status above search; captured condition and columns in the dialog |
| Surface | Japanese labels, native controls, clear pending/error feedback and responsive wrapping |

## Three review rounds

1. Domain, worker/API and migrations: validation, scope, revisions, caps, duplicate names and upgrade/reopen tests.
2. Browser/UI: daily flows, drafts, typed values, hidden-field preservation, CSV scope, pending/error/conflict and keyboard/focus.
3. Independent read-only review and fixes, three-width screenshots, required CI and release API baseline on the final head.

Reproduce concrete findings before adopting fixes. Use at most three improvement rounds; stop after the same failure twice.
Record actual test results, source revisions and unverified limits; never infer browser success from compilation alone.
The user authorized commits, pushes and PR creation for this work. Leave the PR unmerged.
