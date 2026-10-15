# Find records by a field's value

Open **項目の条件で絞り込む** below the text search, select a field, then choose **一致する** or **空欄**.
For equality, enter the typed value; choose **条件を適用** or **検索** to apply it together with text search.
The **適用中** summary describes the fetched result. Changes to controls remain drafts until applied.
Paging and CSV use the applied condition across all matching pages.
**条件を解除** removes only the field condition; **クリア** removes both field and text search.
Selecting another app resets all query controls. A failed request preserves controls for retry and disables export.

## API contract

Both JSON listing and CSV accept optional `filter`, a URL-encoded JSON object:

```json
{"field":"status","value":"完了"}
```

Combine it with `q` using AND; use existing `limit`/`before` on JSON listing only.
For example, `filter=%7B%22field%22%3A%22done%22%2C%22value%22%3Afalse%7D` selects unchecked records.
The field must belong to the selected app; values follow its type:

| Field | Filter value |
| --- | --- |
| Text/multiline | Exact case-sensitive string, up to 128 Unicode characters; whitespace is significant |
| Number | JSON number within the existing safe range; numeric `0` and `0.0` compare equally |
| Checkbox | JSON `true` or `false`, distinct from numbers and strings |
| Date | A valid `YYYY-MM-DD` date |
| Single choice | One exact configured choice |
| Any field | `null` selects missing/null/empty-string cells, excluding whitespace-only text, zero and false |

An equality value of `""` matches explicitly saved empty text; `null` includes missing and null cells too.
Unknown fields/properties, missing field/value, invalid JSON, wrong types, objects/arrays and invalid dates/choices return 400.
The filter is validated in the domain; SQL binds both the key and value without constructing a user-controlled JSON path.
Without `filter`, existing listing/search/export behavior is preserved. No stored-data change is required.

This is one exact/empty condition, not a query language or saved view. It scans JSON fields within the selected app.
No indexed-query performance, range/OR conditions, shared views, or new permissions are claimed.
Existing snapshot CSV limits and [spreadsheet-format restrictions](csv.md) still apply.
See [API v1](api.md) and [the design contract](../.agent-harness/contracts/record-filters.md).
