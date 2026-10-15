# Bootstrap validation

The local app-builder milestone is implemented on `feat/local-app-builder`; [Draft PR 1](https://github.com/haranishi/business-apps-rs/pull/1) remains open for review.
The source is an independent implementation licensed under Apache-2.0.

## Mechanical evidence

| Check | Result |
| --- | --- |
| `scripts/check.sh` | Formatting, Clippy, 13 Rust tests, documentation links/SVGs, and JavaScript syntax passed |
| `cargo +1.88 check --workspace --all-targets --locked` | Passed on macOS arm64; Linux is covered by CI |
| `cargo deny --locked check licenses bans sources` | Passed for supported Linux/macOS targets |
| `npm run test:e2e` | 15 cases passed at 390/768/1440 px using a disposable database |
| Repository skill validator | Passed |
| Japanese README quick document lint | No findings |

[Initial implementation CI](https://github.com/haranishi/business-apps-rs/actions/runs/38594745894) passed for `32418a44315d791e9a63813e89c47bcc15a3c1e1`.
The latest finishing commit's CI is linked by the PR's current checks; an earlier green run is not evidence for that commit.

## Review and actual behavior

An independent read-only code review found unsafe numeric rounding, stale dialog completion, and a browser test's dependency on earlier data.
The implementation now rejects unsafe numeric values, locks pending form operations, and seeds the keyboard case independently.
Regression cases cover these behaviors and delayed app/page responses.

A separate visual reviewer inspected seven screenshots without reading implementation code.
Its two findings—scroll guidance below the mobile table and broken contact strings—were corrected.
The second review scored 19/20 within its stated scope; eight visual categories used images and keyboard/speed categories used the implementation agent's actual operation evidence.

Playwright operated two preview apps and four synthetic records: create, save, search, reopen, and cancel at three widths; page errors were zero.
Additional local UI samples measured save-click to visible row: 15 samples, p50 55.47 ms, p95/max 58.82 ms including automation overhead.
Twenty-four Tab steps stayed within the modal or the native browser boundary, and Escape returned focus to the editor's invoking button at three widths.
Measured labels/buttons in the desktop app-builder dialog had a minimum text contrast of 4.8578.
No screen-reader session or formal accessibility certification was performed.

## Performance and limits

[Measured API baseline](../../docs/performance.md): 10,000 records, eight clients, 500 samples per operation, release mode, SSD database.
The raw result records the timestamp/environment and the measured API source revision.
Text search is a per-app scan; there is no comparative benchmark against kintone.

Authentication, teams, permissions, editable schemas, approvals, and a production deployment remain later milestones.
The preview refuses non-loopback listeners. No package release or PR merge was performed.
