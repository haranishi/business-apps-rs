// SPDX-License-Identifier: Apache-2.0
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { resolve, join } from "node:path";
const root = resolve(import.meta.dirname, "..");
mkdirSync(join(root, ".data"), { recursive: true });
const directory = mkdtempSync(join(root, ".data", "e2e-"));
const child = spawn("cargo", ["run", "--locked", "-p", "business-apps-server"], {
  cwd: root, stdio: "inherit",
  env: { ...process.env, BIZAPP_DB: join(directory, "test.sqlite"), BIZAPP_BIND: `127.0.0.1:${process.env.BIZAPP_TEST_PORT || "8091"}` },
});
let stopping = false;
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { stopping = true; child.kill("SIGINT"); });
child.on("error", (error) => { console.error(error.message); rmSync(directory, { recursive: true, force: true }); process.exit(1); });
child.on("exit", (code) => { rmSync(directory, { recursive: true, force: true }); process.exit(stopping ? 0 : code || 1); });
