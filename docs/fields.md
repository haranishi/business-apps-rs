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
An unchanged textarea keeps its original API-supplied line endings when another field is edited.
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
Format 3 adds `apps.revision` with default 1 for schema conflict checks.
On startup, an owned format-1/2 database advances directly to 3 in a transaction; field JSON, record values, IDs, and record revisions are preserved.
No record rewrite is required. Missing `options` decode as an empty list.
Older binaries refuse newer formats instead of misreading schema rules. Future and unrelated formats are refused.
Before upgrading a database you need to keep, stop the server and copy the database together with any WAL/SHM files.
Keep that stopped copy if you need to return to an older binary; automatic downgrade is unsupported.

App/field labels and order are editable; new optional fields are allowed. See [safe form editing](schema-editing.md).
