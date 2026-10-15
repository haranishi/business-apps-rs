# Field rules and database formats

Field keys stay stable and use ASCII letters, digits, or underscores, starting with a letter.
Apps have 1–32 fields; labels and app names have 1–80 Unicode characters.

| Kind | JSON value | Rules |
| --- | --- | --- |
| `text` | String | At most 4,000 characters; required text cannot be blank |
| `textarea` | String | Same limit, preserving plain multiline content; no HTML interpretation |
| `number` | Number | Finite, within ±9,007,199,254,740,991; ordinary floating-point fractions |
| `boolean` | Boolean | Both false and true satisfy required fields |
| `date` | String | Real Gregorian day, exact `YYYY-MM-DD`, years 0001–9999; no timezone |
| `select` | String | Exactly one of the field's configured `options` |

Missing/null values are allowed for optional fields. Empty dates/selections are sent as null by the UI.
Required values reject missing/null; unknown keys and wrong JSON types are rejected.
A select schema needs 1–50 distinct, trimmed options of 1–80 characters without line breaks.
Other kinds cannot carry nonempty options. Option labels are compared exactly and rendered as plain text.

```json
{"name":"Tasks","fields":[
  {"key":"due","label":"Due date","kind":"date","required":true},
  {"key":"status","label":"Status","kind":"select","options":["Open","Done"]},
  {"key":"notes","label":"Notes","kind":"textarea"}
]}
```

## Saved-data compatibility

SQLite format 1 contains the original text/number/boolean schemas.
Format 2 adds date/select/textarea, with optional `options` metadata in each saved field.
On startup, an owned format-1 database advances to 2 in a transaction; tables, field JSON, values, IDs, and revisions are preserved.
No row rewrite is required. Missing `options` decode as an empty list.
Older binaries refuse format 2 instead of misreading new field kinds. Future and unrelated formats are refused.
Before upgrading a database you need to keep, stop the server and copy the database together with any WAL/SHM files.
Keep that stopped copy if you need to return to an older binary; automatic downgrade is unsupported.

App schemas remain immutable in this milestone. Schema-editing rules will require a separate design and migration.
