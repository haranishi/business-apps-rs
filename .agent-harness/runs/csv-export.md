# CSV export verification

## Scope and review

[Issue 9](https://github.com/haranishi/business-apps-rs/issues/9) and [PR 10](https://github.com/haranishi/business-apps-rs/pull/10) cover bounded, app-scoped spreadsheet export.
No dependencies, stored-data format, public hosting, or package release are added.
Independent read-only standards review found no documented-standard breaches.
Independent specification review identified table replacement during export completion as a focus risk.

## Reproduction and correction

- Initial head `00ecdf7`: [all five CI checks passed](https://github.com/haranishi/business-apps-rs/actions/runs/38734966854), including 63 Chromium cases.
- Added a regression at head `171e3a8` before changing the implementation.
- [Reproduction run](https://github.com/haranishi/business-apps-rs/actions/runs/38735364515): native Linux/macOS, MSRV and dependencies passed; browser focus regression failed at all three widths.
- The delayed export completed while a record draft was open; after Escape, the recreated Edit button did not regain focus.
- Export completion now updates only its control and status, leaving the invoking table button attached.
- The same regression remains in the normal browser suite and verifies the draft as well as focus restoration.

## Evidence and limitations

Rust cases cover encoding/formula prefixes, typed values, schema order, search/isolation and row/byte rejection without an attachment.
Browser flows cover complete multi-page downloads, BOM/filename, applied versus draft text, retry, pending/stale handling and editing focus.
Initial CI screenshots at 390/768/1440 px were inspected; content wraps without horizontal page overflow.
Final head `c399d18` passed [all five CI checks](https://github.com/haranishi/business-apps-rs/actions/runs/38735772900): 36 Rust and 66 Chromium cases.
Final three-width captures received independent visual review with no concrete defect; PR 10 was merged after verification.
Large native/WASM builds ran on GitHub Actions; local checks covered formatting, docs, diff and test JavaScript syntax.
No manual local app session or isolated dynamic security audit ran; source security review found no concrete bypass candidate.
No certification of Excel/LibreOffice behavior or CSV round trips is claimed; [CSV limitations](../../docs/csv.md) apply.
