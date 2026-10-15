# Typed-filter verification

[Issue 11](https://github.com/haranishi/business-apps-rs/issues/11) and [PR 12](https://github.com/haranishi/business-apps-rs/pull/12) cover one exact/empty condition shared by JSON pages and CSV.
Data format, dependencies, revisions, loopback protections and existing Store methods are preserved.

## Independent review and reproduced defects

- Initial head `00507d1` passed [all five CI checks](https://github.com/haranishi/business-apps-rs/actions/runs/38736971179), including 81 Chromium cases.
- Independent standards review found no documented breaches; duplicate native input bounds were an optional maintainability suggestion.
- Specification review identified multiline control normalization and blocked exact empty-text conditions.
- Review also identified a stale next-page cursor after a different condition failed to load.
- Added regressions before changing the implementation at head `baa1dbe`.
- [Reproduction run](https://github.com/haranishi/business-apps-rs/actions/runs/38737391188): those three regressions failed at all three widths; 81 other browser cases passed.
- Corrected the multiline control to a textarea and allowed empty equality for text controls.
- Resetting queries now clears the next cursor; pagination and CSV require a successfully loaded result.
- The original regressions remain in the normal browser suite, including retry and zero/false/null distinctions.

## Verification scope

Domain/API/store tests cover typed validation, equality/empty semantics, literal SQL-like values, app scope, cursor/CSV parity and reopen preservation.
Browser cases cover typed controls, apply/draft/clear, errors/retry, stale responses, markup in app/field/choice text and the three reproduced gaps.
Native Linux/macOS, Rust 1.88, WASM, dependency/advisory policy, Chromium and final screenshots must pass before merge.
The separate release baseline uses 10,000 records, concurrency 8 and 200 samples per workload; raw JSON is retained in Actions.
Its results include client overhead on shared hardware and do not establish stable latency thresholds or cross-machine improvements.
Large builds/tests ran on GitHub Actions; local checks were formatting, docs, JavaScript/Python syntax and diffs.
No manual local app session, spreadsheet certification, screen-reader validation or isolated dynamic security audit is claimed.
