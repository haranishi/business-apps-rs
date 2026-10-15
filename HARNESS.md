# Development harness

A change is complete when another contributor can reproduce its behavior and checks.

## Before implementation

1. State the user problem and acceptance criteria in an Issue or a short contract.
2. Read the relevant domain, API, storage, or UI boundary and [design procedure](docs/development.md).
3. For new screens, describe the user story and the five UX layers in `docs/ui.md`.
4. Discuss new public APIs, breaking migrations, major dependencies, and network deployment in an Issue first.

## Implement and verify

- Keep a change focused; include necessary regression tests with it.
- Run `scripts/check.sh`: generated Rust UI assets, native/WASM Clippy, Rust tests, and documentation links.
- Run `npm ci && npm run test:e2e` for changed browser flows, including real WASM startup/failure recovery.
- Exercise create → save → search → edit → reopen using synthetic data.
- Check 390, 768, and 1440 px layouts, keyboard access, error recovery, and focus.
- Use `scripts/benchmark.py` against a release build when changing hot paths.
- Check `cargo deny check licenses bans sources` before adding dependencies.

New features and data-integrity changes receive an independent read-only review when an authorized reviewer is available.
Review findings must be reproduced before changing accepted behavior.
Use at most three improvement rounds; report an identical failure after two rounds with the remaining work.

## Commit and PR

Review the staged files and exclude databases, secrets, local settings, and artifacts.
PRs explain the user-visible change, how it was verified, and remaining limitations.
A passing CI is evidence for that commit; it does not grant merge or release permission.
Draft PRs may be updated as focused commits pass their checks.

## Evidence

Record commands, outcomes, source revision, and limitations in `.agent-harness/runs/`.
Keep machine-specific paths and private operational notes outside the repository.
The local and CI test environments use temporary databases, never real business records.
