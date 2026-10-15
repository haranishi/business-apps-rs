# Rust browser UI and onboarding validation

Runtime revision: `f9ee3f8`; design [Issue 6](https://github.com/haranishi/business-apps-rs/issues/6).
[PR 7](https://github.com/haranishi/business-apps-rs/pull/7) carries the implementation and contributor documentation.
Final documentation changes are verified separately by [PR CI](https://github.com/haranishi/business-apps-rs/pull/7/checks).

## Reproducible checks

- `scripts/check.sh`: formatting, native/WASM Clippy without warnings, 28 Rust cases, documentation links/SVGs passed.
- `npm run test:e2e`: all 54 Chromium cases passed at 390/768/1440 px with isolated synthetic databases.
- Native `cargo +1.88 check --workspace --all-targets --locked --offline` passed; CI additionally builds/checks WASM on 1.88.
- `cargo deny --offline --locked check licenses bans sources` passed, including the wasm32 graph.
- A fresh `CARGO_BUILD_TARGET_DIR` build and the default build produced byte-identical WASM, glue, and startup assets.
- Missing or mismatched wasm-bindgen CLI versions were refused without modifying generated assets.
- Explicit server compilation embedded the generated assets; an independent check matched the served WASM SHA-256.
- Documentation checks cover links, SVGs and the 100-line development-document limit; Japanese quick lint found no issues.

Local tools: Rust 1.99.0, wasm-bindgen CLI 0.2.129, Node 24.12.0, Playwright 1.64.1, Python 3.14.3, arm64 macOS.
CI uses Node 22, stable Rust on Linux/macOS, and a separate minimum Rust 1.88 job.
The minimum-version job pins `RUSTUP_TOOLCHAIN` so the repository stable override cannot select another compiler.

## Independent review and recovery

The implementation, contributor documentation, and independent code/UI reviews used separate task scopes.
Code review checked saved values, CRLF/CR, zero/false, immutable choices, hidden fields, revisions, stale responses, borrow lifetimes, listeners, CSP, and generation/embedding.
Three reproduced findings were fixed in one improvement round per finding:

1. Dynamic import inside the startup error handler now catches a failed glue download.
2. UI installation failure removes partially mounted controls, releases owned listeners, and retains the recovery message.
3. Cargo metadata resolves the actual target directory, preventing selection of a stale module after a configured-output build.

An independent real-browser recheck passed failed WASM/glue downloads, installation exceptions, initial GET delay, initial 503 and reload recovery.
UI review also caught false empty-state content during initial loading; loading status and an initial creation lock now prevent it.
The mobile welcome heading was shortened after visual review to avoid an isolated trailing phrase.

## Screen and operation evidence

- Independent operation followed the README: create customer fields, save, search, edit, change settings, restart, and reopen saved values.
- Three widths retained table/dialog boundaries without document overflow; table overflow stayed within its scroll container.
- Required-error drafts, keyboard focus/Escape, field reorder, pending-control locks, and conflict/reload flows passed.
- Page errors were zero during normal operation. Startup-failure tests intentionally exercise the error path.
- Four synthetic Rust UI screenshots now show the table, builder, record editor and form settings in the documentation.
- Independent Web/UI evaluation: 19/20; affected operation experience: 20/20, based on the reviewed synthetic flows.
- Screen-reader behavior and native operating-system choice popups remain outside this headless Chromium review.

## Build size and remaining scope

Rust UI release output, thin LTO, one codegen unit, stripped; wasm-bindgen `--target web`, without an additional WASM optimizer:

| Delivered generated file | Uncompressed bytes |
| --- | ---: |
| WASM module | 299,761 |
| JavaScript bindings | 31,555 |
| Startup module | 421 |

These file sizes do not measure loading or interaction latency, and establish no speed advantage.
Application logic uses Rust; authored CSS, generated JavaScript, Playwright tooling and CI configuration remain.
API v1, SQLite format 3 and stored record revisions are unchanged. No data-format migration is required.
This is a loopback-only development preview; team identity/permissions, packaged distribution and supported deployment remain future work.
