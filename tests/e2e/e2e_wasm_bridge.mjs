// e2e_wasm_bridge.mjs - WP-F2 gate: a Rust export is callable from Node.
//
// This is the whole point of the WASM path. `crates/domain` logic has to
// produce identical results on the server and in the browser, and this test is
// the first link in that chain: compile Rust to wasm32, generate glue with
// wasm-bindgen, load it in a JavaScript runtime, call it, and compare against
// a value Rust owns.
//
// It lives in tests/e2e/ rather than tests/ because it needs a real build
// artifact on disk, which puts it outside the fast pytest lane by the rule in
// docs/E2E_TESTS.md.
//
// Run: ./pipeline/build_wasm.sh && node tests/e2e/e2e_wasm_bridge.mjs

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
}).trim();

const bridgePath = path.join(repoRoot, "dist_wasm", "node", "ple_bridge.js");

if (!fs.existsSync(bridgePath)) {
  console.error(`FAIL: ${bridgePath} missing.`);
  console.error("Build it first: ./pipeline/build_wasm.sh");
  process.exit(1);
}

const bridge = await import(bridgePath);

// The Rust side returns its own CARGO_PKG_VERSION. Comparing against the
// version in crates/wasm/Cargo.toml proves the value crossed the boundary
// rather than being produced by the glue.
const cargoToml = fs.readFileSync(path.join(repoRoot, "crates", "wasm", "Cargo.toml"), "utf8");
const inheritsWorkspaceVersion = /version\.workspace\s*=\s*true/.test(cargoToml);
const workspaceToml = fs.readFileSync(path.join(repoRoot, "Cargo.toml"), "utf8");
const versionSource = inheritsWorkspaceVersion ? workspaceToml : cargoToml;
const versionMatch = versionSource.match(/^version\s*=\s*"([^"]+)"/m);
assert.ok(versionMatch, "could not read the crate version from Cargo.toml");
const expectedVersion = versionMatch[1];

const actualVersion = bridge.bridge_version();
assert.equal(
  actualVersion,
  expectedVersion,
  `bridge_version() returned ${actualVersion}, expected ${expectedVersion}`,
);

console.log(`PASS: wasm bridge_version() returned ${actualVersion} from Node`);
