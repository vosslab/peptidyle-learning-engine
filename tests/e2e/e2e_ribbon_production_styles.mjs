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

const css = fs.readFileSync(path.join(repoRoot, "dist/main.css"), "utf8");
assert.match(css, /\.ple-app-ribbon(?:[,{])/u, "production CSS includes the Ribbon root rule");
assert.match(
  css,
  /--ple-ribbon-block-size/u,
  "production CSS includes the fixed Ribbon block-size token",
);
