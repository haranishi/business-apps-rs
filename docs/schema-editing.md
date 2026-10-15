# Safe form editing

After creating an app, open **フォーム設定** (Form settings) above its record table.
Rename the app or labels, move fields with the up/down buttons, or add optional fields.
Save to update the form; existing record JSON and record revisions remain unchanged.

| Change | Allowed? | Reason |
| --- | --- | --- |
| App name or field labels | Yes | Presentation changes do not reinterpret values |
| Field order | Yes | Values remain keyed by stable field keys |
| Add optional fields | Yes | Existing records may omit them; the editor shows an empty input |
| Remove or rename a field key | No | Could orphan saved values |
| Change a type, required flag, or option list | No | Needs a separate data migration and recovery design |
| Add a required field | No | Existing records lack that value |

The existing fields' incompatible controls are locked in the UI; the domain independently rejects these changes through the API.
New field keys avoid all existing keys, including keys created through the API.
An app still has at most 32 fields.

## App revisions and conflict recovery

Apps have their own revision, separate from record revisions.
GET `/api/v1/apps/{app_id}` returns the current schema and revision.
PUT the same URL with the full `name`, `fields`, and expected `revision`.

```mermaid
sequenceDiagram
  participant A as Editor A
  participant B as Editor B
  participant API as Rust API
  participant DB as SQLite worker
  A->>API: Save form, revision 1
  API->>DB: Validate additive change, compare revision
  DB-->>A: Updated form, revision 2
  B->>API: Save another form, revision 1
  API-->>B: 409; keep draft and offer explicit reload
```

A successful atomic update increments the app revision. Concurrent writers have exactly one winner.
A rejected or stale update does not change the app or its records.
The browser keeps a conflicting draft; **最新版を読み込む** explicitly replaces it with the current schema.
Save-in-flight locks inputs, movement, cancel, and Escape until the request finishes.
A delayed settings load cannot replace a newer dialog or an app selection.
Older tabs keep their displayed form until reloaded; edits preserve every loaded record value, including added fields absent from that form.
Saving settings returns to the first record page with the new schema and clears the previous search.

## SQLite format 3

Startup migrates owned format-1/2 databases in a transaction by adding `apps.revision INTEGER NOT NULL DEFAULT 1`.
Existing field JSON, record JSON, IDs, and record revisions are preserved; records are not rewritten.
Failure rolls the migration back. New databases start at format 3; future and unrelated databases are refused.
Older binaries refuse format 3. Stop the server and keep a complete database/WAL/SHM copy before upgrading data you need to preserve.
Restore that stopped copy to return to an older binary; automatic downgrade is unsupported.

Domain tests cover the mutation matrix. API/storage tests cover revision conflicts, one-winner concurrency, isolation, exact stored JSON, upgrades, failure rollback, and reopen.
Browser tests cover form evolution, old-record editing, key collisions, keyboard movement, pending saves, and conflict recovery at three widths.
