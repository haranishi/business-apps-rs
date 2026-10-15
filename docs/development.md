# Design one useful change before implementing it

Use this guide for a feature or behavior change; use [CONTRIBUTING](../CONTRIBUTING.md) for setup and a first small fix.
The output is a focused Issue or short contract another contributor can review and test.

## Start with a user and an observable result

Write who is doing what, why it matters, and what success looks like in the UI or API.
For example: an app owner adds an optional due date after saving work; old records reopen with their original values and an empty date.
Include the current behavior, desired behavior, smallest synthetic example, and what is outside the change.
Check the [roadmap](roadmap.md) so a local preview feature does not quietly become a team/security platform.

## Make the UX and compatibility decisions

For a new screen or changed flow, record the five layers before implementation:

| Layer | Question to answer |
| --- | --- |
| Strategy | Whose task becomes easier, and why? |
| Requirements | Which actions and data are essential for this result? |
| Structure | Which screen owns the action, and where does the user go next? |
| Skeleton | Where are inputs, status, primary actions, and recovery? |
| Surface | How do labels, native controls, focus, and small screens express those choices? |

Use [UI stories and diagrams](ui.md) as the current baseline.
Name the contracts affected by the proposal:

| Boundary | Preserve or explicitly change |
| --- | --- |
| Domain | Field types, optional/required meaning, validation, stable keys |
| API | Request/response shapes, errors, app isolation, record/app revisions |
| Storage | Existing values and identities, database format, reopen and recovery |
| Browser | Drafts, keyboard/focus, pending locks, stale responses, three widths |
| Build | Rust/WASM targets, generated assets, server embedding, reproducible setup |

State the chosen approach, one relevant alternative, and why the choice fits the current scope.
A migration needs an upgrade/reopen and recovery plan; a public API or major dependency needs an Issue discussion first.
The [architecture](design.md) and [API](api.md) describe the current boundaries.

## Write acceptance checks before the implementation

Checks should describe the user's outcome and the failure that would lose data or block the task.
For the optional-date example:

1. Create an app and save a record before adding the field.
2. Add an optional date; reopen the old record and confirm original values and an empty date.
3. Save a valid date, reload the browser, and reopen it without a timezone shift.
4. Exercise invalid input, competing settings, an older open form, and pending writes without losing saved or draft values.
5. Check 390/768/1440 px and the relevant keyboard/focus path.

Choose domain, storage/API, WASM, and browser checks that cover the affected boundaries.
Use [testing](testing.md) for commands and evidence; a docs-only correction can stay with the docs check.

## Implement and review one coherent result

Keep domain rules independent from HTTP/DOM/SQLite, storage work on its worker, and browser logic in Rust.
Build the UI before compiling the server so the embedded assets reflect the change.
Update the relevant diagrams/examples with contract changes and keep each Markdown file within 100 lines.

For new behavior or data-integrity changes, request an independent read-only review with the goal, scope, acceptance criteria, and results.
Reproduce findings before changing accepted behavior; record what was fixed and what remains unverified.
Follow the [project harness](../HARNESS.md) for improvement rounds and evidence.

A PR should explain the user-visible result, compatibility, commands/results, and remaining limits.
CI checks an exact revision; maintainers decide acceptance and merge. Package publication and deployment are separate decisions.

## Keep the design small enough to maintain

Use the existing entry points until a real task requires a new framework, plugin boundary, or deployment mode.
Record deferred choices with their reason and the evidence that would reopen them.
Protect saved work and repeatable operations as the project grows toward team permissions, views, and workflows.
