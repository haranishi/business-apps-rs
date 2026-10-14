#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Measure a release server using a disposable synthetic database."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import http.client
import json
import math
import os
from pathlib import Path
import platform
import signal
import socket
import sqlite3
import subprocess
import tempfile
import threading
import time
from urllib.parse import quote


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--records", type=int, default=10000)
    parser.add_argument("--requests", type=int, default=500)
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if min(args.records, args.requests, args.concurrency) < 1:
        parser.error("record, request, and concurrency counts must be positive")
    root = Path(__file__).resolve().parents[1]
    binary = Path(os.environ.get("CARGO_TARGET_DIR", root / "target")) / "release" / "business-apps-server"
    if not binary.is_file():
        parser.error("build the release server with cargo build --release --locked -p business-apps-server")
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    local = threading.local()
    connections = []
    connection_lock = threading.Lock()

    def send(path, method="GET", data=None):
        if not hasattr(local, "connection"):
            local.connection = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
            with connection_lock:
                connections.append(local.connection)
        headers = {"X-Bizapp-Request": "1"}
        if data is not None:
            headers["Content-Type"] = "application/json"
        local.connection.request(method, path, json.dumps(data) if data is not None else None, headers)
        response = local.connection.getresponse()
        body = response.read()
        if response.status >= 400:
            raise RuntimeError(f"benchmark request failed: HTTP {response.status}")
        return json.loads(body) if body else None

    with tempfile.TemporaryDirectory(prefix="business-apps-benchmark-") as temporary:
        database = Path(temporary) / "bench.sqlite"
        env = {**os.environ, "BIZAPP_DB": str(database), "BIZAPP_BIND": f"127.0.0.1:{port}"}
        process = subprocess.Popen([str(binary)], cwd=root, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            deadline = time.monotonic() + 15
            while True:
                if process.poll() is not None:
                    raise RuntimeError("benchmark server exited before readiness")
                try:
                    send("/health")
                    break
                except (OSError, http.client.HTTPException):
                    local.connection.close()
                    if time.monotonic() > deadline:
                        raise RuntimeError("benchmark server did not become ready")
                    time.sleep(0.05)
            app = send("/api/v1/apps", "POST", {"name": "Synthetic benchmark", "fields": [
                {"key": "name", "label": "Name", "kind": "text", "required": True},
                {"key": "quantity", "label": "Quantity", "kind": "number"},
                {"key": "active", "label": "Active", "kind": "boolean"},
            ]})
            with sqlite3.connect(database) as seed:
                seed.executemany("INSERT INTO records(app_id, revision, values_json) VALUES (?, 1, ?)",
                    ((app["id"], json.dumps({"name": f"Customer {i:05d}", "quantity": i, "active": True})) for i in range(args.records)))
            prefix = f"/api/v1/apps/{app['id']}/records"
            workloads = {
                "record_read": lambda i: send(f"{prefix}/{i % args.records + 1}"),
                "first_page_30": lambda i: send(f"{prefix}?limit=30"),
                "text_scan_no_match": lambda i: send(f"{prefix}?limit=30&q={quote('not-present')}") ,
                "record_write": lambda i: send(prefix, "POST", {"values": {"name": f"New {i}", "quantity": i, "active": False}}),
            }
            results = {}
            with ThreadPoolExecutor(max_workers=args.concurrency) as workers:
                for name, operation in workloads.items():
                    for i in range(min(args.requests, 30)):
                        operation(i)
                    def measured(i):
                        started = time.perf_counter()
                        operation(i)
                        return (time.perf_counter() - started) * 1000
                    started = time.perf_counter()
                    samples = sorted(workers.map(measured, range(args.requests)))
                    elapsed = time.perf_counter() - started
                    results[name] = {"samples": len(samples), "p50_ms": samples[math.ceil(len(samples) * .5) - 1],
                        "p95_ms": samples[math.ceil(len(samples) * .95) - 1], "p99_ms": samples[math.ceil(len(samples) * .99) - 1],
                        "requests_per_second": len(samples) / elapsed}
            result = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), "build": "release",
                "machine": {"os": platform.system(), "architecture": platform.machine(), "logical_cpus": os.cpu_count()},
                "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                "initial_records": args.records, "concurrency": args.concurrency, "sqlite": "WAL / FULL synchronous",
                "scope": "end-to-end localhost HTTP, including Python client overhead; text search scans string values",
                "workloads": results}
            if args.output:
                args.output.parent.mkdir(parents=True, exist_ok=True)
                args.output.write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result, indent=2))
        finally:
            for connection in connections:
                connection.close()
            if process.poll() is None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()
