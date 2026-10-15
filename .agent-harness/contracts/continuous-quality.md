# Continuous quality checks

## Problem and scope

Dependency license checks alone do not detect known vulnerable packages.
Successful browser runs also need inspectable evidence before a UI change is merged.
Keep the existing five required check names and read-only workflow permissions.

## Acceptance

- Run Rust advisory/license/source checks and reject high/critical npm advisories.
- Repeat checks weekly, with one run per PR update and a merged-main run.
- Retain only synthetic browser evidence for seven days, including successful runs.
- Preserve native Linux/macOS, Rust 1.88, WASM, and all existing browser flows.
- Review the workflow diff and confirm all five checks on the exact PR head.

No application, dependency, database, or public API changes are included.
No package release or deployment is authorized by this workflow.
Use at most three improvement rounds, stopping after an identical failure twice.
