import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const inventoryPath = new URL("./browser_mock_behavior_allocation.json", import.meta.url);
const marker =
  /(?:src\/)?api\/mock|PLE_BROWSER_TEST_TRANSPORT|browser_client_browser_test|local_development_browser_test|dist_browser_test|helper_browser_test_server|__PLE_|page\.route|context\.route|route\.(?:fulfill|abort)|addInitScript|mock.*(?:capture|fixture|state)|(?:mock|fixture).*capture/u;
const roots = [
  "crates/project-tools/src",
  "devel",
  "pipeline",
  "src",
  "tests",
  "playwright.config.ts",
];
const excluded = new Set([
  "tests/browser_mock_behavior_allocation.json",
  "tests/test_browser_mock_behavior_allocation.mjs",
]);
const requiredArchitecturalConsumers = new Set([
  "crates/project-tools/src/fixtures.rs",
  "devel/dist_clean.sh",
  "src/api/browser_client_browser_test.ts",
  "src/auth/local_development_browser_test.tsx",
  "tests/test_editor_ui.mjs",
]);

function sourceFiles(entry) {
  if (excluded.has(entry)) return [];
  const stat = fs.statSync(entry);
  if (stat.isFile()) return [entry];
  return fs
    .readdirSync(entry, { withFileTypes: true })
    .flatMap((child) => sourceFiles(path.join(entry, child.name)));
}

function markerConsumers() {
  const files = roots.flatMap(sourceFiles);
  return files
    .filter((file) => !file.startsWith("src/api/mock/"))
    .filter(
      (file) =>
        requiredArchitecturalConsumers.has(file) ||
        marker.test(file) ||
        marker.test(fs.readFileSync(file, "utf8")),
    )
    .map((file) => file.replaceAll(path.sep, "/"))
    .sort();
}

test("mock-runtime marker consumers exactly match the future-owner allocation", () => {
  const inventory = JSON.parse(fs.readFileSync(inventoryPath, "utf8"));
  const allocated = inventory.runtimeMarkerAllocations.map((entry) => entry.path).sort();
  assert.deepEqual(markerConsumers(), allocated);
  for (const entry of inventory.runtimeMarkerAllocations) {
    assert.match(entry.owner, /^(?:I1|L1|A1|S1|V1|F1|X1|R1)$/u);
    assert.ok(entry.behavior.length > 0, entry.path);
  }
});

test("retained narrow unit evidence remains outside the mock runtime graph", () => {
  const inventory = JSON.parse(fs.readFileSync(inventoryPath, "utf8"));
  for (const file of inventory.retainedUnitEvidence) {
    assert.doesNotMatch(fs.readFileSync(file, "utf8"), /src\/api\/mock\//u, file);
  }
});
