// The shipped browser has one production-authentication composition path.
// This slower artifact check belongs to the E2E tier rather than the fast
// Node unit-test lane because it inspects the emitted browser artifact.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";

import { browserFontUrlsFromStylesheet } from "../../src/browser_font_assets.mjs";

const repoRoot = process.cwd();

execFileSync("npm", ["run", "build"], { cwd: repoRoot, stdio: "pipe" });
const indexHtml = fs.readFileSync("dist/index.html", "utf8");
const browserBundle = fs.readFileSync("dist/main.js", "utf8");
const browserStylesheet = fs.readFileSync("dist/main.css", "utf8");
const embedStylesheet = fs.readFileSync("dist/styles/ple_embed.css", "utf8");

assert.match(
  indexHtml,
  /<link rel="stylesheet" href="\/main\.css\?v=[0-9a-f]{8}"\s*\/>/u,
  "the production build fingerprints the shared frontend environment stylesheet",
);
assert.match(
  browserStylesheet,
  /@media\s*\(prefers-reduced-motion:\s*reduce\)/u,
  "the stylesheet loaded by production contains the reduced-motion accessibility rules",
);
for (const fontUrl of browserFontUrlsFromStylesheet(`${browserStylesheet}\n${embedStylesheet}`)) {
  assert.ok(fs.existsSync(`dist${fontUrl}`), `production CSS font URL is delivered: ${fontUrl}`);
}
const environmentMarkers = [
  browserStylesheet.indexOf("--ple-surface:"),
  browserStylesheet.search(/@media\s*\(max-width:\s*48rem\)/u),
  browserStylesheet.search(/\[data-product-role=["']?instructor["']?\]/u),
  browserStylesheet.search(/prefers-reduced-motion\s*:\s*reduce/u),
  browserStylesheet.indexOf("--ple-ribbon-top-block-size:"),
];
assert.ok(
  environmentMarkers.every((position) => position >= 0),
  "each environment layer is emitted",
);
assert.deepEqual(
  environmentMarkers,
  [...environmentMarkers].sort((left, right) => left - right),
  "production main.css preserves global, responsive, role, accessibility, and app-shell order",
);
assert.match(
  indexHtml,
  /<script type="module" src="\/main\.js\?v=[0-9a-f]{8}"><\/script>/u,
  "the production module resolves from the gateway root",
);
assert.doesNotMatch(
  browserBundle,
  /(?:\/api\/auth\/login|local-login\.txt|local-development-credential)/u,
  "the production browser contains no local credential login transport or UI",
);
