# HTTP API v1

The preview listens on loopback only and has no user/permission model.
All data-changing requests require `X-Bizapp-Request: 1` and JSON bodies where applicable.
The server accepts only local Host values and same-origin browser requests.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/health` | Process health; does not inspect the database |
| GET | `/api/v1/apps` | List app schemas |
| POST | `/api/v1/apps` | Create an immutable app schema |
| GET | `/api/v1/apps/{app_id}/records` | List records using `limit`, `before`, and optional `q` |
| POST | `/api/v1/apps/{app_id}/records` | Create a validated record |
| GET | `/api/v1/apps/{app_id}/records/{id}` | Read one record within its app |
| PUT | `/api/v1/apps/{app_id}/records/{id}` | Replace values if `revision` matches |
| DELETE | `/api/v1/apps/{app_id}/records/{id}?revision=N` | Delete if `revision` matches |

## Create an app and record

```sh
curl -fsS http://127.0.0.1:8080/api/v1/apps \
  -H 'Content-Type: application/json' -H 'X-Bizapp-Request: 1' \
  -d '{"name":"Customers","fields":[{"key":"name","label":"Name","kind":"text","required":true}]}'

curl -fsS http://127.0.0.1:8080/api/v1/apps/1/records \
  -H 'Content-Type: application/json' -H 'X-Bizapp-Request: 1' \
  -d '{"values":{"name":"Example customer"}}'
```

App keys contain ASCII letters, digits, and underscores and start with a letter.
Supported `kind` values are `text`, `number`, and `boolean`.
Unknown fields, wrong types, missing required values, and whitespace-only required text are rejected.
Optional null values are accepted. A required boolean accepts both true and false.
Schemas have 1–32 fields; labels/names have 1–80 characters; text values have at most 4,000 characters.
A record body is bounded to 256 KiB. Listing returns at most 100 records.

## Paging, search, and revisions

A page is `{"records":[...],"next_cursor":123}`; use `before=123` for the next page.
IDs are descending. `next_cursor` is null at the end.
Search matches literal substrings of string values, case-insensitively for ASCII; it is not a query language.
SQLite's built-in lowercasing does not provide full Unicode case folding.
A record returns `id`, `app_id`, `revision`, and `values`.
Replace a record with `{"revision":1,"values":{"name":"Updated"}}`; success returns revision 2.
A stale revision returns 409. Reload before choosing whether to overwrite the latest values.

## Errors

Domain errors return `{"error":{"code":"validation","message":"..."}}`.
400 means invalid input, 404 means an app/record is missing, 409 means a revision conflict.
Host/origin failures return 403; unavailable storage returns 503 without internal database details.
Malformed JSON, invalid path/query values, and oversized bodies use Axum's standard rejection responses.
API v1 is a preview contract; breaking changes require an explicit design note and migration discussion.
