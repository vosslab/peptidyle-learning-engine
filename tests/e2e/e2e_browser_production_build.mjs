// The shipped browser has one production-authentication composition path.
// This slower artifact check belongs to the E2E tier rather than the fast
// Node unit-test lane because it inspects the emitted browser artifact.

import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";

const repoRoot = process.cwd();

execFileSync("npm", ["run", "build"], { cwd: repoRoot, stdio: "pipe" });
const indexHtml = fs.readFileSync("dist/index.html", "utf8");
const browserBundle = fs.readFileSync("dist/main.js", "utf8");

assert.match(
  indexHtml,
  /<link rel="stylesheet" href="\/style\.css\?v=[0-9a-f]{8}"\s*\/>/u,
  "the production build fingerprints the shared stylesheet",
);
const accessibilityStylesheet = fs.readFileSync("dist/styles/accessibility.css", "utf8");
const accessibilityHash = crypto
  .createHash("sha256")
  .update(accessibilityStylesheet)
  .digest("hex")
  .slice(0, 8);
assert.match(
  indexHtml,
  new RegExp(
    `<link rel="stylesheet" href="/styles/accessibility\\.css\\?v=${accessibilityHash}"\\s*/>`,
    "u",
  ),
  "the production build fingerprints the extracted accessibility stylesheet",
);
assert.match(
  accessibilityStylesheet,
  /@media \(prefers-reduced-motion: reduce\)/u,
  "the production artifact contains the reduced-motion accessibility rules",
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
