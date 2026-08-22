import assert from "node:assert/strict";
import test from "node:test";

import {
  UI_CORPUS_MANIFEST,
  manifestArtifactPaths,
  validateManifest,
} from "./playwright/ui_corpus_manifest.ts";

test("the TypeScript facade projects the validated JSON corpus", () => {
  validateManifest();
  assert(UI_CORPUS_MANIFEST.length > 0);
  assert.deepEqual(
    UI_CORPUS_MANIFEST.map((artifact) => artifact.captureOrder),
    Array.from({ length: UI_CORPUS_MANIFEST.length }, (_, index) => index + 1),
  );
  assert.equal(new Set(manifestArtifactPaths()).size, UI_CORPUS_MANIFEST.length);
});

test("the facade preserves nested journeys and declared privacy evidence", () => {
  const responsive = UI_CORPUS_MANIFEST.filter(
    (artifact) => artifact.stateId === "assignment_overview",
  );
  assert(responsive.length > 0);
  assert(
    UI_CORPUS_MANIFEST.every(
      (artifact) =>
        artifact.path.startsWith("docs/screenshots/") &&
        artifact.privacyChecks[0] === "no_private_material",
    ),
  );
});
