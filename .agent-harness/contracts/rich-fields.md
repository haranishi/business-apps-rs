# Rich business fields

Design: [Issue 2](https://github.com/haranishi/business-apps-rs/issues/2).

A task owner needs a due date, a controlled status, and multiline notes so the app can represent routine work.
The loop is choose field types → configure choices → save typed values → reopen and edit without loss.

| UX layer | Decision |
| --- | --- |
| Strategy | Let local task/customer apps represent dates, status, and notes |
| Requirements | Date/select/textarea, validation, persisted upgrade; schemas remain immutable |
| Structure | Extend the existing builder and editor, retaining cancel/conflict recovery |
| Skeleton | Choice input appears below its field row; multiline notes use a full-width textarea |
| Surface | Reuse current typography, focus styles, and native form controls |

## Acceptance

- Dates are real Gregorian days in years 0001–9999, stored without timezone conversion.
- Selects accept exactly one configured option; 1–50 distinct trimmed options, each 1–80 characters.
- Plain text/textarea retain the 4,000-character limit; no rich HTML is introduced.
- Missing/null optional values work; required date/select values cannot be empty.
- Invalid schemas/records are rejected before persistence.
- Upgrade owned version-1 SQLite metadata to version 2 transactionally, keeping rows and revisions.
- Reopen both old and new data; reject future/unrelated databases.
- Browser checks cover the new controls and pending-write locks at three widths.
- Run existing checks, license policy, independent read-only review, and required CI before merge.

The user has authorized commits, pushes, PR creation, and merges for this project.
Deployment and package releases remain outside this change.
