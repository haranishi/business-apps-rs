# Named record views

A saved view belongs to one app and stores a name, text query, typed condition and ordered visible columns.
It makes a recurring daily list reusable without changing the app's schema or record values.
Column visibility is presentation, not an access-control boundary; record editors and CSV retain all fields.

## API and constraints

| Method | Path suffix under `/api/v1/apps/{app_id}` | Result |
| --- | --- | --- |
| GET | `/views` | Array of saved views, newest first |
| POST | `/views` | Create a definition; returns 201 |
| GET | `/views/{view_id}` | Read one app-scoped view |
| PUT | `/views/{view_id}` | Replace definition using current revision |
| DELETE | `/views/{view_id}?revision=N` | Delete using current revision; records remain intact |

Create with a definition:

```json
{"name":"未対応","query":"東京","filter":{"field":"done","value":false},"columns":["done","name"]}
```

Responses contain `id`, `app_id`, `revision`, and `definition`. Updates send `{"revision":1,"definition":{...}}`.
Definitions require `name` and `columns`; omitted `query` defaults to empty and omitted `filter` to null.
Use 1–80 characters for a trimmed name, 0–128 Unicode characters for query, and 1–32 distinct current field keys for columns.
Names must be distinct within an app (exact case-sensitive match); an app has at most 50 saved views.
Filter rules are the existing [typed-filter contract](record-filters.md). Unknown properties are rejected.
Domain input/cap violations return 400; invalid JSON shapes return Axum's 422 rejection.
Wrong app/view IDs return 404; stale revisions return 409.
Duplicate names return 409 with `error.code=duplicate_view_name`; the cap uses `error.code=view_limit`.
The existing mutation header, loopback, Host/Origin/Fetch-Metadata, body-size and `no-store` protections apply.

## Browser workflow

Use **この条件で一覧を保存** after applying a search/condition, choose columns and their order, and enter a name.
Unsubmitted search/filter input is excluded. Select a saved name to recall it; **すべてのレコード（標準）** resets conditions and columns.
**一覧の設定** updates the selected definition using the currently applied conditions, or deletes only that definition.
Definitions persist; the current selection resets on app switches or a page reload.
Failed saves keep name/column drafts. Revision conflicts offer explicit reload, which replaces the draft only when requested.
Pending writes/reloads lock controls and Escape; cancel returns focus to the invoking action.
View-catalog failures have their own retry and do not block record editing or CSV.
Hidden columns remain in record editors and the full-schema CSV; visibility does not restrict access to data.
API-created CRLF/CR multiline conditions keep their exact value when an unchanged textarea is resubmitted.

## Stored-data upgrade and recovery

This feature introduces SQLite format 4 and one app-scoped `record_views` table.
Formats 1/2/3 upgrade in one transaction; fields, record values, IDs and existing revisions remain unchanged.
An unexpected migration failure rolls back. Foreign databases and unknown/newer formats are refused.
Concurrent opens inspect and migrate under the same SQLite write lock.

Before opening an existing database with the new binary, create a consistent SQLite backup and retain the older binary.
For example, SQLite's CLI `.backup` command produces a consistent copy, including committed WAL data:

```sh
sqlite3 .data/business-apps.sqlite ".backup '.data/before-saved-views.sqlite'"
```

The old binary refuses format 4. To revert, stop the server and restore the retained backup with the older binary.
Changes made after that backup would not be present in the restored copy. Do not reset `user_version` to bypass this guard.
See [SQLite's backup documentation](https://sqlite.org/backup.html) and [the design contract](../.agent-harness/contracts/saved-views.md).
