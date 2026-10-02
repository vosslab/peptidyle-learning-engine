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
const indexHtml = fs.readFileSync(path.join(repoRoot, "dist/index.html"), "utf8");
const bundlePath = indexHtml.match(/src="\/(main\.[0-9a-f]{8}\.js)"/u)?.[1];
const stylesheetPath = indexHtml.match(/href="\/(main\.[0-9a-f]{8}\.css)"/u)?.[1];
assert.ok(bundlePath, "the production index references its fingerprinted browser bundle");
assert.ok(stylesheetPath, "the production index references its fingerprinted stylesheet");
const browserArtifacts = [
  indexHtml,
  fs.readFileSync(path.join(repoRoot, "dist", bundlePath), "utf8"),
  fs.readFileSync(path.join(repoRoot, "dist", stylesheetPath), "utf8"),
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
