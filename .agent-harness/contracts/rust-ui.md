# Rust browser UI and contributor onboarding

[Issue 6](https://github.com/haranishi/business-apps-rs/issues/6) tracks this change.
A contributor should see what the app does, start it from a clean clone, and understand how to design and verify a change.
The user also wants app code, including the browser UI, written in Rust.

## Decisions and boundaries

- Port layout and behavior to `crates/ui` using wasm-bindgen/web-sys and shared domain models.
- Keep CSS and generated JavaScript glue/bootstrap; browser tests/CI tooling are separate from app code.
- Preserve API v1, SQLite format 3, origin guards, typed validation, revisions, and all existing flows.
- Generate assets explicitly with `cargo xtask build-web`; embed them in one server binary, exclude them from Git.
- Add only `wasm-unsafe-eval` to CSP; do not permit general eval or inline scripts.
- Prefer a direct DOM port to preserve native dialog/focus behavior; keep event listener lifetimes bounded.
- Document Rust 1.88, pinned build tooling, a first-app walkthrough, capabilities/limits, and a concrete design procedure.

## UX layers

| Layer | Decision |
| --- | --- |
| Strategy | Let local business-app authors and contributors work in Rust |
| Requirements | Keep six types, CRUD/search, safe form editing, and recovery |
| Structure | Keep the current app navigation, builder, table, and native dialogs |
| Skeleton | Preserve control IDs/labels and focus destinations; add an honest startup status |
| Surface | Keep current responsive CSS and real screenshots |

## Completion evidence

- Clean asset generation; WASM-target and native Clippy/build/tests; minimum Rust 1.88 checks.
- Dependency licenses/sources include wasm32; generated file sizes and build conditions recorded.
- All existing browser cases at 390/768/1440 px plus actual WASM loading/MIME/CSP and failure recovery.
- Preserve all loaded record values, CRLF/CR, immutable choices, zero/false, drafts, revisions, and request tokens.
- Inspect screens and keyboard flows, with independent read-only code/UI review.
- Check README commands/links and development Markdown length; use at most three improvement rounds.

Commits, pushes, PRs, sub-agent work, and merges are authorized. Deployments and package releases are outside scope.
