// e2e_ribbon_icon_sprite_build.mjs - production delivery evidence for the Ribbon SVG sprite.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

import { getRepoRoot } from "../../devel/repo_root.mjs";

const repoRoot = getRepoRoot();
const spriteBuildScript = path.join(repoRoot, "devel/build_ribbon_icon_sprite.mjs");
const pipelineBuildScript = path.join(repoRoot, "pipeline/build.mjs");

execFileSync("node", ["--import", "tsx", spriteBuildScript, "--check"], {
  cwd: repoRoot,
  stdio: "inherit",
});

execFileSync("node", [pipelineBuildScript, "--skip-wasm"], {
  cwd: repoRoot,
  stdio: "inherit",
});

const sourceSprite = fs.readFileSync(
  path.join(repoRoot, "src/ribbon/assets/ribbon-icons.svg"),
  "utf8",
);
const builtSprite = fs.readFileSync(path.join(repoRoot, "dist/assets/ribbon-icons.svg"), "utf8");
const browserArtifacts = [
  fs.readFileSync(path.join(repoRoot, "dist/index.html"), "utf8"),
  fs.readFileSync(path.join(repoRoot, "dist/main.js"), "utf8"),
  fs.readFileSync(path.join(repoRoot, "dist/main.css"), "utf8"),
  builtSprite,
].join("\n");

assert.equal(
  builtSprite,
  sourceSprite,
  "the production build copies the exact checked Ribbon sprite",
);
assert.doesNotMatch(
  browserArtifacts,
  /https?:\/\/[^\s"']*(?:fontawesome|fortawesome)/iu,
  "the production Ribbon icon delivery has no Font Awesome CDN or remote reference",
);
