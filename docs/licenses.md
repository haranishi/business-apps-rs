# Licensing and provenance

Original code, documentation, diagrams, and test examples in this repository use Apache-2.0.
`LICENSE` contains the complete license; `NOTICE` identifies the project and attribution boundary.
Commercial use and modification are permitted under the license conditions; the patent grant has its defined scope.
Contributions are submitted under the same license, and contributors must have the right to submit them.
A contributor agreement or ownership transfer is not required for ordinary contributions.

## Independent implementation

This is an original business-app platform, independent of Cybozu and kintone.
Do not copy proprietary source, screenshots, icons, logos, templates, or confidential specifications.
References to another product identify the comparison; they do not grant trademark rights or imply endorsement.
The working name is descriptive and can be revisited before a branded release.

## Dependencies and distributions

- `Cargo.lock` records the reviewed dependency graph.
- `deny.toml` permits listed open-source licenses and blocks unknown/unlicensed packages.
- Run `cargo deny check licenses bans sources` before introducing a dependency.
- For a binary distribution, collect dependency license texts, copyright/NOTICE files, and an SBOM for that exact build.
- SQLite's bundled upstream code is in the public domain; rusqlite and libsqlite3-sys have their own package licenses.
- Do not label a release as fully reviewed merely because package SPDX metadata passed a check.

No binary or package release is produced by the initial CI.

## Primary references

- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [Apache licensing FAQ](https://www.apache.org/foundation/license-faq.html)
- [SQLite copyright statement](https://sqlite.org/copyright.html)
- [Cybozu kintone product-name/logo guideline](https://cybozu.co.jp/logotypes/assets/kintone/kintone-visual-identity-guide.pdf)
