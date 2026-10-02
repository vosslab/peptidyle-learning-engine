// The Ribbon stylesheet is deliberately reachable from the live browser entry
// before the application shell mounts the component. This artifact check stays in the E2E tier
// because it verifies the emitted production CSS rather than a source import.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

import { getRepoRoot } from "../../devel/repo_root.mjs";

const repoRoot = getRepoRoot();
const pipelineBuildScript = path.join(repoRoot, "pipeline/build.mjs");

execFileSync("node", [pipelineBuildScript, "--skip-wasm"], {
  cwd: repoRoot,
  stdio: "inherit",
});

const indexHtml = fs.readFileSync(path.join(repoRoot, "dist/index.html"), "utf8");
const stylesheetPath = indexHtml.match(/href="\/(main\.[0-9a-f]{8}\.css)"/u)?.[1];
assert.ok(stylesheetPath, "the production index references its fingerprinted stylesheet");
const css = fs.readFileSync(path.join(repoRoot, "dist", stylesheetPath), "utf8");
assert.match(css, /\.ple-app-ribbon(?:[,{])/u, "production CSS includes the Ribbon root rule");
assert.match(
  css,
  /--ple-ribbon-block-size/u,
  "production CSS includes the fixed Ribbon block-size token",
);
