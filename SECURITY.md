# Security policy

This is a local development preview. It refuses non-loopback listeners and checks Host/Origin headers.
It has no user authentication, permission model, or isolation between people sharing a machine.
Do not use it as a publicly reachable service or for real confidential records.

## Reporting

Use GitHub's private vulnerability reporting on this repository's Security tab.
Do not put credentials, real records, or an exploit containing private data in a public Issue.
If private reporting is unavailable, open a public Issue asking for a private contact without disclosing the vulnerability.

Describe the affected revision, environment, minimal synthetic reproducer, and expected boundary.
There is currently no guaranteed response deadline or production support commitment.

Network deployment requires an authentication/authorization design and corresponding tests before removing the loopback restriction.
