# Roadmap

The first milestone establishes a reliable local app/record loop and a contributor-friendly repository.
The project does not yet offer production team collaboration or kintone API compatibility.

| Stage | Outcome | Required evidence |
| --- | --- | --- |
| Local preview | Typed app schemas, persistent CRUD, revision conflicts, browser UI | Domain/API/reopen/browser tests and a measured baseline |
| Team foundation | Login, memberships, app/record permissions, audit history | Authorization matrix, isolation tests, threat model |
| Rich schemas | Schema migrations, select/date/exact-decimal fields, views, import/export | Old-record migration tests and recovery procedures |
| Indexed queries | Typed filters, indexed sorting and full-text search | Query plans, realistic workloads, latency/memory results |
| Workflow | State transitions, approvals and integrations | Permission/state-machine tests and idempotent delivery |
| Deployment | Supported self-hosting, backup/restore and packaged releases | Restore tests, notices/SBOM, and supported-platform CI |

## First contributions

- Add a regression for a Japanese search edge case and document the actual behavior.
- Improve a narrow screen-reader or keyboard flow with a browser test.
- Translate the existing UI labels while keeping the same operations.
- Add a reproducible performance workload before proposing a storage optimization.

Open an Issue with the expected result, relevant files, and how to check it.
Maintainers currently review asynchronously; there is no guaranteed response deadline.
