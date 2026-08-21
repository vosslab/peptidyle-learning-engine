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
];
const disabledInput = "src/auth/local_development_disabled.tsx";
const liveBrowserClient = "src/api/browser_client.ts";
const browserTestClient = "src/api/browser_client_browser_test.ts";
const browserTestLogin = "src/auth/local_development_browser_test.tsx";
const browserTestInputs = [
  browserTestClient,
  browserTestLogin,
  "src/api/mock/client.ts",
  "src/api/mock/local_development_auth.ts",
];

function normalizedBuildInputs(metafilePath) {
  const metafile = JSON.parse(fs.readFileSync(metafilePath, "utf8"));
  return new Set(
    Object.keys(metafile.inputs).map((input) =>
      path.relative(repoRoot, path.resolve(repoRoot, input)).split(path.sep).join("/"),
    ),
  );
}

function buildBrowser(localDevelopmentAuth, assetBase = "root", browserTestTransport = false) {
  const outputDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "ple-browser-build-"));
  const metafilePath = path.join(outputDirectory, "build-metafile.json");
  try {
    execFileSync("node", ["pipeline/build.mjs", "--skip-wasm", `--asset-base=${assetBase}`], {
      cwd: repoRoot,
      env: {
        ...process.env,
        PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH: localDevelopmentAuth ? "1" : "0",
        PLE_BROWSER_TEST_TRANSPORT: browserTestTransport ? "1" : "0",
        PLE_BROWSER_OUTPUT_DIRECTORY: outputDirectory,
        PLE_BROWSER_METAFILE_PATH: metafilePath,
      },
      stdio: "pipe",
    });
    return {
      inputs: normalizedBuildInputs(metafilePath),
      indexHtml: fs.readFileSync(path.join(outputDirectory, "index.html"), "utf8"),
      hasBrowserTestAssets: fs.existsSync(path.join(outputDirectory, "api", "assets")),
    };
  } finally {
    fs.rmSync(outputDirectory, { recursive: true, force: true });
  }
}

const production = buildBrowser(false);
const local = buildBrowser(true);
const githubPages = buildBrowser(false, "relative");
const browserTest = buildBrowser(true, "root", true);

assert.equal(
  production.inputs.has(disabledInput),
  true,
  "production omitted its capability-free boundary",
);
assert.equal(local.inputs.has(disabledInput), false, "local build retained the disabled boundary");
for (const input of localOnlyInputs) {
  assert.equal(production.inputs.has(input), false, `production graph retained ${input}`);
  assert.equal(local.inputs.has(input), true, `local build graph omitted ${input}`);
}
assert.equal(production.inputs.has(liveBrowserClient), true, "live build omitted its HTTP client");
assert.equal(local.inputs.has(liveBrowserClient), true, "local build omitted its HTTP client");
for (const input of browserTestInputs) {
  assert.equal(production.inputs.has(input), false, `live graph retained ${input}`);
  assert.equal(local.inputs.has(input), false, `local live graph retained ${input}`);
  assert.equal(browserTest.inputs.has(input), true, `browser-test graph omitted ${input}`);
}
assert.equal(production.hasBrowserTestAssets, false, "live build retained browser-test assets");
assert.equal(local.hasBrowserTestAssets, false, "local live build retained browser-test assets");
assert.equal(browserTest.hasBrowserTestAssets, true, "browser-test build omitted its assets");

assert.match(
  production.indexHtml,
  /<(?:link|script)\b[^>]+(?:href|src)="\/(?:style|main)\.css\?v=[0-9a-f]{8}"/u,
  "the live build must resolve browser assets from the gateway root",
);
assert.match(
  production.indexHtml,
  /<script type="module" src="\/main\.js\?v=[0-9a-f]{8}"><\/script>/u,
  "the live module must resolve from the gateway root",
);
assert.doesNotMatch(
  githubPages.indexHtml,
  /(?:href|src)="\/(?:style|main)\.(?:css|js)\?v=/u,
  "the GitHub Pages variant must retain project-relative assets",
);
