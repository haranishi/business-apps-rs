# Local business-app builder

## Purpose

Enable a user to define an app from typed fields and manage persistent records through a browser.
Provide a contribution-ready Rust OSS repository with diagrams, a development skill, and CI.

## Scope

Allowed: this repository's Rust crates, browser assets, tests, examples, documentation, and CI.
Excluded: copying proprietary code/assets, existing personal projects, real customer data, and global settings.
The initial app schema is immutable. Authentication, teams, permissions, workflows, and cloud deployment follow later.

## Acceptance

- Text, number, and checkbox fields support required-value validation.
- Records can be created, paged, searched, edited, and deleted within their own app.
- Revision conflicts prevent lost updates; reopening the database preserves saved records.
- The server refuses non-loopback listeners and rejects cross-origin requests.
- A contributor can follow the README, tests, and PR guide without private configuration.
- Architecture/UI diagrams and a repository skill describe the implemented boundaries.
- Linux/macOS Rust checks and browser tests run in GitHub Actions.

## Validation

Run Rust domain/API/storage tests, Clippy, formatting, documentation checks, and browser flows.
Inspect the UI at 390/768/1440 px and use a release-build benchmark with synthetic data.
Confirm GitHub checks against the final PR head, not an earlier commit.

## Operations

The commissioning user authorized commits, pushes, and PR creation for this work.
Merging, package publication, and hosting a public service are separate decisions.
Maximum improvement rounds: three; identical failure after two rounds is returned with evidence.
