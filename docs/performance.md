# Local performance baseline

The first measurements establish a baseline for this implementation, not a comparison with kintone or another product.

## Workload and environment

Measured API source: `32418a44315d791e9a63813e89c47bcc15a3c1e1`.
Rust 1.99.0 release build, Apple M4 Pro / arm64 / 14 logical CPUs, macOS (Darwin), and a database on the external SSD development volume.
The harness used a disposable local database with 10,000 synthetic records, three typed fields, eight clients, 30 warmups, and 500 samples per operation.
SQLite ran in WAL mode with its FULL synchronous default. No durability setting was weakened.
The numbers include Python client and localhost HTTP overhead.

| Operation | p50 ms | p95 ms | p99 ms | Requests/s |
| --- | ---: | ---: | ---: | ---: |
| Indexed record read | 0.41 | 0.93 | 1.39 | 16,367 |
| First page, 30 records | 0.51 | 1.37 | 1.77 | 12,636 |
| Text search, no match | 25.62 | 27.12 | 37.05 | 307 |
| Validated record write | 0.73 | 3.28 | 10.28 | 6,000 |

See [the raw result](benchmarks/local-baseline.json) for full precision and the UTC timestamp.
The source reference identifies the measured API; later UI-only changes do not alter its storage/query paths.

## Reproduce it

```sh
cargo build --release --locked -p business-apps-server
python3 scripts/benchmark.py --records 10000 --requests 500 --concurrency 8 --output .artifacts/benchmark.json
```

Record your machine and environment when comparing results. A single run does not establish sustained throughput, capacity, multi-user behavior, or a speedup against another product.
These small in-memory response bodies and loopback traffic are not representative of network deployment.
The Rust server currently serializes database work on one bounded worker; queueing can dominate latency under heavier concurrency.

## Next optimization

The largest measured cost is per-app text scanning. Typed query indexes or full-text search should be evaluated with realistic filters and update workloads.
Exact substring semantics, Unicode handling, app isolation, and durable writes must remain explicit when changing that path.
Use before/after workloads and query plans rather than replacing SQLite solely because of this baseline.
