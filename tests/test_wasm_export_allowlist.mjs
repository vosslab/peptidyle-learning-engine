// WP-C6 gate: every processed WebAssembly export requires explicit review.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const EXPECTED_EXPORTS = [
  { name: "__abort_handler", kind: "global" },
  { name: "__externref_table_dealloc", kind: "function" },
  { name: "__instance_terminated", kind: "global" },
  { name: "__wbindgen_externrefs", kind: "table" },
  { name: "__wbindgen_free", kind: "function" },
  { name: "__wbindgen_malloc", kind: "function" },
  { name: "__wbindgen_realloc", kind: "function" },
  { name: "__wbindgen_start", kind: "function" },
  { name: "bridge_version", kind: "function" },
  { name: "memory", kind: "memory" },
  { name: "preview_native_draft", kind: "function" },
  { name: "timer_verdict", kind: "function" },
  { name: "validate_assignment_config", kind: "function" },
  { name: "validate_response_format", kind: "function" },
  { name: "verify_presentation_descriptor", kind: "function" },
];

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
}).trim();
const rawModule = path.join(
  repoRoot,
  "target",
  "wasm32-unknown-unknown",
  "debug",
  "wasm_bridge.wasm",
);
const outputDirectory = path.join(repoRoot, "generated", "wasm-export-check");
const processedModule = path.join(outputDirectory, "ple_boundary_bg.wasm");

function compareExport(left, right) {
  return left.name.localeCompare(right.name) || left.kind.localeCompare(right.kind);
}

test("processed WebAssembly exports match the reviewed allowlist", () => {
  execFileSync(
    "cargo",
    [
      "build",
      "--quiet",
      "--target",
      "wasm32-unknown-unknown",
      "-p",
      "wasm_bridge",
      "--features",
      "web",
    ],
    { cwd: repoRoot, stdio: "inherit" },
  );
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "-p",
      "project-tools",
      "--",
      "bindgen",
      rawModule,
      "web",
      outputDirectory,
      "ple_boundary",
    ],
    { cwd: repoRoot, stdio: "inherit" },
  );

  const bytes = fs.readFileSync(processedModule);
  const wasmModule = new WebAssembly.Module(bytes);
  const actual = WebAssembly.Module.exports(wasmModule).toSorted(compareExport);
  const expected = EXPECTED_EXPORTS.toSorted(compareExport);

  assert.deepEqual(
    actual,
    expected,
    "WebAssembly export list changed; review the diff and update the allowlist deliberately",
  );
});
