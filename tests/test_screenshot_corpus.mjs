import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CANONICAL_VIEWPORTS,
  ROLE_IDS,
  decodeManifest,
  loadManifest,
} from "./playwright/screenshot_corpus/manifest";
import {
  createReceipt,
  inspectCorpus,
  pngDimensions,
  promoteCorpus,
  receiptJson,
  renderAtlas,
  verifyPublishedArtifacts,
} from "./playwright/screenshot_corpus/publication";
import { PRIVACY_PROFILES } from "./playwright/screenshot_corpus/privacy_profiles";

const repositoryRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const manifestPath = path.join(repositoryRoot, "docs/screenshots/current_capture_manifest.json");

function oneCaptureManifest(manifest) {
  const capture = manifest.captures[0];
  assert.ok(capture);
  return { ...manifest, captures: [capture] };
}

function fixturePng(capture, salt) {
  const viewport = CANONICAL_VIEWPORTS[capture.viewport];
  const identity = Buffer.from(`${salt}:${capture.id}`, "utf8");
  const bytes = Buffer.alloc(24 + identity.length);
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]).copy(bytes);
  bytes.write("IHDR", 12, "ascii");
  bytes.writeUInt32BE(viewport.width, 16);
  bytes.writeUInt32BE(viewport.height, 20);
  identity.copy(bytes, 24);
  return bytes;
}

async function writeFixtureCorpus(root, manifest, salt) {
  await Promise.all(ROLE_IDS.map((role) => mkdir(path.join(root, role), { recursive: true })));
  await Promise.all(
    manifest.captures.map(async (capture) => {
      await writeFile(path.join(root, capture.path), fixturePng(capture, salt));
    }),
  );
}

function expectedPrivacyProfile(capture) {
  if (capture.role === "public") return "public";
  if (capture.role === "instructor") return "instructor_answer_free";
  if (capture.role === "sysadmin") {
    return capture.scenario === "sysadmin_support" ? "sysadmin_scoped_roster" : "sysadmin_account";
  }
  if (capture.scenario === "student_authorization") return "authorization_denial";
  if (capture.checkpoint === "response_selected") return "student_selected_response";
  if (
    (capture.scenario === "student_assignment_history" &&
      capture.checkpoint === "selected_history") ||
    (capture.scenario === "student_assignment_attempt" && capture.checkpoint === "submitted")
  ) {
    return "student_feedback_released";
  }
  if (
    capture.scenario === "student_assignment_overviews" ||
    capture.checkpoint.startsWith("question_unanswered_") ||
    capture.checkpoint === "numerical_response" ||
    capture.checkpoint === "hotspot_response"
  ) {
    return "student_unanswered";
  }
  return "student_self";
}

test("manifest decoding rejects procedural fields and traversal", async () => {
  const source = JSON.parse(await readFile(manifestPath, "utf8"));
  const procedural = structuredClone(source);
  const captures = procedural["captures"];
  captures[0]["selector"] = "button";
  assert.throws(() => decodeManifest(procedural), /invalid fields/u);

  const traversal = structuredClone(source);
  traversal["captures"][0]["path"] = "../escape.png";
  assert.throws(() => decodeManifest(traversal), /flat semantic filename/u);
});

test("the shipped manifest selects the least-data profile for each semantic surface", async () => {
  const manifest = await loadManifest(manifestPath);
  for (const capture of manifest.captures) {
    assert.equal(capture.privacyProfile, expectedPrivacyProfile(capture), capture.id);
    assert.ok(PRIVACY_PROFILES[capture.privacyProfile]);
  }
  assert.equal(PRIVACY_PROFILES.student_unanswered.selectedControl, "forbidden");
  assert.equal(PRIVACY_PROFILES.student_selected_response.selectedControl, "required");
  assert.equal(PRIVACY_PROFILES.student_selected_response.statusHeading, "forbidden");
});

test("PNG dimensions derive from image bytes", () => {
  const header = Buffer.alloc(24);
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]).copy(header);
  header.write("IHDR", 12, "ascii");
  header.writeUInt32BE(393, 16);
  header.writeUInt32BE(852, 20);
  assert.deepEqual(pngDimensions(header), { width: 393, height: 852 });
});

test("published verification rejects a receipt that no longer binds its PNG", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-receipt-"));
  try {
    const screenshotRoot = path.join(root, "screenshots");
    const manifest = oneCaptureManifest(await loadManifest(manifestPath));
    await writeFixtureCorpus(screenshotRoot, manifest, "receipt");
    const images = await inspectCorpus(screenshotRoot, manifest);
    const digest = "d".repeat(64);
    const receipt = createReceipt(digest, images);
    const receiptPath = path.join(screenshotRoot, "current_capture_receipt.json");
    const atlasPath = path.join(root, "SCREENSHOT_ATLAS.md");
    await writeFile(
      path.join(screenshotRoot, "current_capture_manifest.json"),
      `${JSON.stringify(manifest)}\n`,
      "utf8",
    );
    await writeFile(receiptPath, receiptJson(receipt), "utf8");
    await writeFile(atlasPath, renderAtlas(manifest, "screenshots/"), "utf8");
    await verifyPublishedArtifacts({
      screenshotRoot,
      manifest,
      manifestDigest: digest,
      receiptPath,
      atlasPath,
    });

    const tampered = structuredClone(receipt);
    tampered.images[0].sha256 = "e".repeat(64);
    await writeFile(receiptPath, receiptJson(tampered), "utf8");
    await assert.rejects(
      verifyPublishedArtifacts({
        screenshotRoot,
        manifest,
        manifestDigest: digest,
        receiptPath,
        atlasPath,
      }),
      /does not bind/u,
    );
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("publication restores the prior corpus when a role replacement fails", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-promotion-"));
  try {
    const screenshotRoot = path.join(root, "docs", "screenshots");
    const stagingRoot = path.join(root, "test-results", "staging");
    const atlasPath = path.join(root, "docs", "SCREENSHOT_ATLAS.md");
    const manifest = oneCaptureManifest(await loadManifest(manifestPath));
    const digest = "f".repeat(64);
    await writeFixtureCorpus(screenshotRoot, manifest, "old");
    const oldImages = await inspectCorpus(screenshotRoot, manifest);
    const oldReceipt = receiptJson(createReceipt(digest, oldImages));
    const oldAtlas = "previous atlas\n";
    await writeFile(
      path.join(screenshotRoot, "current_capture_manifest.json"),
      `${JSON.stringify(manifest)}\n`,
      "utf8",
    );
    await writeFile(path.join(screenshotRoot, "current_capture_receipt.json"), oldReceipt, "utf8");
    await writeFile(atlasPath, oldAtlas, "utf8");
    await writeFixtureCorpus(stagingRoot, manifest, "new");
    const oldPng = await readFile(path.join(screenshotRoot, manifest.captures[0].path));

    await assert.rejects(
      promoteCorpus(
        { stagingRoot, screenshotRoot, atlasPath, manifest, digest },
        async (source, target) => {
          if (path.basename(target) === "student") throw new Error("injected replacement failure");
          await rename(source, target);
        },
      ),
      /injected replacement failure/u,
    );

    assert.deepEqual(await readFile(path.join(screenshotRoot, manifest.captures[0].path)), oldPng);
    assert.equal(
      await readFile(path.join(screenshotRoot, "current_capture_receipt.json"), "utf8"),
      oldReceipt,
    );
    assert.equal(await readFile(atlasPath, "utf8"), oldAtlas);
    assert.ok((await stat(path.join(root, "test-results", "publication-backup"))).isDirectory());
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
