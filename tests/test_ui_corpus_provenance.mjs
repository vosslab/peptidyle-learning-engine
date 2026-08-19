// Durable corpus checks use a temporary copy so failure evidence never alters committed screenshots.

import assert from "node:assert/strict";
import { cp, mkdtemp, open, rm } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { requiredVisualEvidenceIssues } from "./playwright/ui_corpus_provenance.mjs";
import { UI_CORPUS_MANIFEST } from "./playwright/ui_corpus_manifest.ts";

const REPOSITORY_ROOT = path.resolve(import.meta.dirname, "..");

async function copiedCorpusRoot() {
  const root = await mkdtemp("/private/tmp/ple-ui-corpus-test.");
  const source = path.join(REPOSITORY_ROOT, "docs", "screenshots");
  const destination = path.join(root, "docs", "screenshots");
  await cp(source, destination, { recursive: true });
  return root;
}

function studentAccessArtifact(surfaceName, viewport) {
  const surface = UI_CORPUS_MANIFEST.find((candidate) => candidate.surface === surfaceName);
  if (surface === undefined) throw new Error(`missing corpus surface: ${surfaceName}`);
  const artifact = surface.artifacts.find((candidate) => candidate.viewport === viewport);
  if (artifact === undefined) throw new Error(`missing ${viewport} artifact for ${surfaceName}`);
  return artifact.path;
}

test("a changed member of the student capture generation is rejected", async () => {
  const root = await copiedCorpusRoot();
  try {
    const destination = studentAccessArtifact("studentAllowedAssignmentOverview", "iphonePro");
    const replacement = studentAccessArtifact("studentInstructorRouteDenial", "iphonePro");
    await cp(path.join(root, replacement), path.join(root, destination));
    const issues = await requiredVisualEvidenceIssues(root);
    assert.ok(issues.some((issue) => issue.includes("committed bytes disagree")));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a required committed PNG with wrong dimensions is rejected", async () => {
  const root = await copiedCorpusRoot();
  try {
    const artifact = studentAccessArtifact("studentAllowedAssignmentOverview", "square");
    const target = path.join(root, artifact);
    const handle = await open(target, "r+");
    try {
      const changedWidth = Buffer.alloc(4);
      changedWidth.writeUInt32BE(799);
      await handle.write(changedWidth, 0, changedWidth.length, 16);
    } finally {
      await handle.close();
    }
    const issues = await requiredVisualEvidenceIssues(root);
    assert.ok(issues.some((issue) => issue.includes("committed PNG dimensions")));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
