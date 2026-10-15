# Working on business-apps-rs

Read `HARNESS.md`, `docs/design.md`, and the relevant API/UI document before changing behavior.

- Keep domain rules independent of HTTP and SQLite.
- Execute SQLite work on its dedicated worker, never on a Tokio runtime thread.
- Preserve app isolation and revision checks on every record mutation.
- Bind only to loopback during this preview. Network access requires a separate authentication design.
- Use parameterized SQL and render user content with DOM text APIs.
- Run `scripts/check.sh` and the affected browser tests before reporting completion.
- Changes to persisted data require a migration and a reopen test.
- Record performance with workload, build mode, machine, and sample count.
- Keep one reviewable purpose per commit/PR; use English PR titles and descriptions.
- Keep credentials, real business data, private instructions, and generated artifacts out of Git.
- Publish, push, merge, or release only within the user's explicit authorization.

The repository skill is `.agents/skills/business-apps/SKILL.md`.
