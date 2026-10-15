# Safe app-form editing

Design: [Issue 4](https://github.com/haranishi/business-apps-rs/issues/4).
A local app owner wants to add an optional due date and reorder fields after recording work, keeping every existing value.

| UX layer | Decision |
| --- | --- |
| Strategy | Evolve a small business app without recreating its data |
| Requirements | Rename app/labels, reorder, append optional fields, detect conflicting schema edits |
| Structure | Form settings opens the existing builder with a saved app revision |
| Skeleton | Stable keys and locked type/required/options, up/down actions, explicit conflict reload |
| Surface | Reuse native controls, disabled states, focus styles, and pending-save messages |

## Integrity and acceptance

- GET/PUT `/api/v1/apps/{app_id}` expose and compare app revisions.
- Existing keys, kinds, required flags, and choice lists cannot change or disappear.
- New fields must be optional; existing records retain their original JSON, IDs, and revisions.
- Concurrent schema updates have exactly one winner; stale writes return 409 and keep the UI draft.
- Renaming labels and ordering fields changes presentation, not stored values.
- Migration to SQLite format 3 adds `apps.revision` with default 1 transactionally from owned formats 1/2.
- Verify old-record identity, reopen, rejected mutations, API isolation, and migration rollback.
- Exercise settings → rename/add/reorder → save → edit old record → reload at 390/768/1440 px.
- Check keyboard movement/focus, conflict recovery, and all controls/cancel/Escape during a pending save.
- Run independent read-only reviews, affected checks and a release benchmark before required CI and merge.

Commits, pushes, PRs, and merges are authorized. Deployment and releases are separate work.
