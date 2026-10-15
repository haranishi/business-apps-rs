# Bounded spreadsheet export

[Issue 9](https://github.com/haranishi/business-apps-rs/issues/9) records the public API design before implementation.
An app owner needs to take complete search results into a spreadsheet without copying pages.

## Boundaries and decisions

- Add a read-only CSV route and Rust/WASM download action using the existing app-scoped literal search.
- Use one SQLite transaction and bounded output; reject above 10,000 rows or 16 MiB without a partial attachment.
- Preserve all saved values and JSON contracts; no migration, CSV dependency, import, or deployment.
- Quote/escape CSV syntax and neutralize formula-like string/header prefixes while retaining typed numbers.
- Use ID-based filenames and explicitly describe CSV as a spreadsheet format, not a backup.

## UX layers

| Layer | Decision |
| --- | --- |
| Strategy | Let an operator take saved work into a spreadsheet |
| Requirements | Complete applied-search export, progress, errors, retry |
| Structure | Existing record workspace, without a new screen |
| Skeleton | Export action and scope hint below search, with its own status |
| Surface | Japanese labels, native button/focus, wrapping at narrow widths |

## Acceptance and evidence

- Check quoting, Japanese/multiline text, formulas, headers, null/false/zero/negative values and schema order.
- Verify app isolation, identical search semantics, empty results, row/byte boundaries and absence of partial CSV.
- Exercise a download across pages, applied versus unsubmitted search, errors/retry, pending lock and stale responses.
- Preserve create/save/search/edit/delete/reopen, conflict handling, drafts and safe form editing.
- Run native Linux/macOS, minimum Rust 1.88, WASM, dependency policy and Chromium at 390/768/1440 px.
- Inspect screenshots and obtain independent read-only review before merge.
- Use at most three improvement rounds; stop after an identical failure twice.

External API integrations, production authentication, deployment, and package releases are outside this change.
