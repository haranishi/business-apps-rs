# Take an app's records into a spreadsheet

Choose **CSVを出力** in the record workspace. The download includes every matching page.
It uses the last submitted search; typing a new search without submitting does not change its scope.
The button reports progress, completion, and errors. After an error, narrow the search and retry.
A table reload (including app/search/page changes) discards an older pending export.

## Format and safety decisions

- `GET /api/v1/apps/{app_id}/records.csv` accepts only optional `q`, with the same literal search as JSON listing.
- Rows use descending record IDs; columns are ID, Revision, then current field labels in schema order.
- UTF-8 BOM supports Japanese spreadsheet workflows; row endings are CRLF.
- Every cell is quoted; embedded quotes are doubled; multiline content remains inside its cell.
- Missing/null values become empty cells; booleans are `true`/`false`; dates retain `YYYY-MM-DD`.
- Formula-like text and labels receive an apostrophe, including whitespace/control-padded prefixes and fullwidth variants.
- Typed numbers retain their value, including negatives; text resembling a number may display according to spreadsheet settings.

This format is for spreadsheets. Formula protection changes some strings, and empty/null/missing values are indistinguishable.
It is not a lossless database backup or a promise of spreadsheet round-trip fidelity. CSV import is not implemented.
Spreadsheet applications may interpret CSV differently; use their text import options when exact display matters.

## Completeness, limits, and privacy

The schema and records come from one SQLite read transaction, scoped to the selected app.
The server builds a bounded response before offering a download, rather than silently truncating it.
More than 10,000 records or 16 MiB of encoded CSV returns 413 with `error.code=export_limit` and no CSV attachment.
Limits include the BOM, header, delimiters, escaped quotes, and row endings.
An empty result still exports its header. A missing app returns 404; invalid search/query parameters return 400.

The existing loopback, Host/Origin/Fetch-Metadata protections and `no-store` apply.
Downloaded files contain the selected app's data; treat them according to your own data-handling requirements.
This feature adds no authentication, external storage, third-party integration, or public sharing.

See [API v1](api.md) and [the export design contract](../.agent-harness/contracts/csv-export.md).
