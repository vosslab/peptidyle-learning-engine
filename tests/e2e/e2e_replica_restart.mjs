#!/usr/bin/env node
/** Public compatibility facade for the fixed live-demo replica oracle owner. */

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const OWNER = fileURLToPath(new URL("e2e_live_demo_service_owner.py", import.meta.url));

if (process.argv.length !== 2) {
  throw new Error("replica restart E2E accepts no command-line arguments");
}

const result = spawnSync("python3", [OWNER, "replica_restart"], {
  cwd: REPO_ROOT,
  env: process.env,
  stdio: "inherit",
});
if (result.error !== undefined) throw result.error;
if (result.signal !== null) throw new Error(`replica restart owner ended on ${result.signal}`);
process.exitCode = result.status ?? 1;
