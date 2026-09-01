// Every processed WebAssembly export requires explicit security review. This
// check builds Rust and runs bindgen, so it belongs in the E2E tier rather than
// the fast Node unit-test lane.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const expectedExports = [
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
  { name: "preview_ple_draft", kind: "function" },
  { name: "question_attempt_timing_decision", kind: "function" },
  { name: "validate_assignment_config", kind: "function" },
  { name: "validate_response_format", kind: "function" },
  { name: "verify_presentation_descriptor", kind: "function" },
];

const repoRoot = process.cwd();
const rawModule = path.join(
  repoRoot,
  "target",
  "wasm32-unknown-unknown",
  "debug",
  "wasm_bridge.wasm",
);

function compareExport(left, right) {
  return left.name.localeCompare(right.name) || left.kind.localeCompare(right.kind);
}

const outputDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "ple-wasm-export-check-"));
try {
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

  const processedModule = path.join(outputDirectory, "ple_boundary_bg.wasm");
  const bytes = fs.readFileSync(processedModule);
  const wasmModule = new WebAssembly.Module(bytes);
  const actual = WebAssembly.Module.exports(wasmModule).toSorted(compareExport);
  const expected = expectedExports.toSorted(compareExport);

  assert.deepEqual(
    actual,
    expected,
    "WebAssembly export list changed; review the diff and update the allowlist deliberately",
  );
} finally {
  fs.rmSync(outputDirectory, { recursive: true, force: true });
}
