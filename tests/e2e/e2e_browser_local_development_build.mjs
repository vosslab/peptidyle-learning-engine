// The local credential boundary is a build capability, not a hidden runtime
// branch. This slower build-artifact check belongs in the E2E tier rather than
// the fast Node unit-test lane.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = process.cwd();
const localOnlyInputs = [
  "src/auth/local_development.tsx",
  "src/api/http_client/local_development_auth.ts",
  "src/api/mock/local_development_auth.ts",
];
const disabledInput = "src/auth/local_development_disabled.tsx";

function normalizedBuildInputs(metafilePath) {
  const metafile = JSON.parse(fs.readFileSync(metafilePath, "utf8"));
  return new Set(
    Object.keys(metafile.inputs).map((input) =>
      path.relative(repoRoot, path.resolve(repoRoot, input)).split(path.sep).join("/"),
    ),
  );
}

function buildBrowser(localDevelopmentAuth) {
  const outputDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "ple-browser-build-"));
  const metafilePath = path.join(outputDirectory, "build-metafile.json");
  try {
    execFileSync("node", ["pipeline/build.mjs", "--skip-wasm"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH: localDevelopmentAuth ? "1" : "0",
        PLE_BROWSER_OUTPUT_DIRECTORY: outputDirectory,
        PLE_BROWSER_METAFILE_PATH: metafilePath,
      },
      stdio: "pipe",
    });
    return normalizedBuildInputs(metafilePath);
  } finally {
    fs.rmSync(outputDirectory, { recursive: true, force: true });
  }
}

const production = buildBrowser(false);
const local = buildBrowser(true);

assert.equal(
  production.has(disabledInput),
  true,
  "production omitted its capability-free boundary",
);
assert.equal(local.has(disabledInput), false, "local build retained the disabled boundary");
for (const input of localOnlyInputs) {
  assert.equal(production.has(input), false, `production graph retained ${input}`);
  assert.equal(local.has(input), true, `local build graph omitted ${input}`);
}
