import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CANONICAL_VIEWPORTS,
  ROLE_IDS,
  VIEWPORT_IDS,
  decodeManifest,
  loadManifest,
} from "./playwright/screenshot_corpus/manifest";
import { SCREENSHOT_SCENARIOS } from "./playwright/screenshot_corpus/scenario_registry";
import {
  createReceipt,
  inspectCorpus,
  pngDimensions,
  receiptJson,
  renderAtlas,
  verifyPublishedArtifacts,
  writeReplayArtifacts,
} from "./playwright/screenshot_corpus/publication";
import {
  SCREENSHOT_GALLERY_FOLDERS,
  verifyScreenshotGalleries,
  writeScreenshotGalleries,
} from "./playwright/screenshot_corpus/screenshot_galleries";
import { PRIVACY_PROFILES } from "./playwright/screenshot_corpus/privacy_profiles";
import {
  reportScenarioThemeVariety,
  scenarioCourseTheme,
} from "./playwright/screenshot_corpus/scenario_theme";

const repositoryRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const manifestPath = path.join(repositoryRoot, "docs/screenshots/current_capture_manifest.json");

const LAPTOP_ONLY_ROLES = new Set(["instructor", "sysadmin"]);
const PUBLIC_VIEWPORTS = ["laptop", "phone"];

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
      await mkdir(path.dirname(path.join(root, capture.path)), { recursive: true });
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
  if (capture.checkpoint.startsWith("response_selected_")) return "student_selected_response";
  if (
    (capture.scenario === "student_assignment_history" &&
      capture.checkpoint.startsWith("selected_history_")) ||
    (capture.scenario === "student_progress_response_stats_history" &&
      capture.checkpoint.startsWith("latest_feedback_")) ||
    (capture.scenario === "student_assignment_attempt" &&
      capture.checkpoint.startsWith("submitted_"))
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
  assert.throws(() => decodeManifest(traversal), /semantic filename/u);
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

test("Course theme comparisons render the same Instructor Course workspace", async () => {
  const manifest = await loadManifest(manifestPath);
  const themeSamples = manifest.captures.filter(
    (capture) => capture.scenario === "instructor_theme_samples",
  );
  assert.ok(themeSamples.length > 0);
  assert.deepEqual(new Set(themeSamples.map((capture) => capture.role)), new Set(["instructor"]));
  assert.deepEqual(
    new Set(themeSamples.map((capture) => capture.routeId)),
    new Set(["courseAssessments"]),
  );
  for (const capture of themeSamples) {
    assert.equal(capture.area, "courses", capture.id);
    assert.equal(capture.workflow, "Course theme comparison", capture.id);
    assert.equal(capture.viewport, "laptop", capture.id);
    assert.match(capture.path, /^instructor\/theme_sample-[a-z0-9-]+\.png$/u, capture.id);
    assert.ok(
      capture.gallery.caption.startsWith("Instructor Course workspace rendered with the "),
      capture.id,
    );
  }
});

test("the screenshot matrix captures each role at its required viewport scope", () => {
  for (const scenario of SCREENSHOT_SCENARIOS) {
    const capturedViewports = [
      ...new Set(scenario.captures.map((capture) => capture.viewport)),
    ].sort();
    if (LAPTOP_ONLY_ROLES.has(scenario.role)) {
      assert.deepEqual(capturedViewports, ["laptop"], scenario.id);
    } else if (scenario.role === "student") {
      const directlyCaptured = [...VIEWPORT_IDS]
        .filter((viewport) => scenario.viewportCoverage[viewport].status === "captured")
        .sort();
      assert.deepEqual(capturedViewports, directlyCaptured, scenario.id);
      for (const viewport of VIEWPORT_IDS) {
        const coverage = scenario.viewportCoverage[viewport];
        if (coverage.status === "captured") {
          assert.ok(
            scenario.captures.some((capture) => capture.viewport === viewport),
            `${scenario.id}:${viewport} is marked captured without a capture`,
          );
        } else {
          assert.ok(
            scenario.captures.some((capture) => capture.checkpoint === coverage.target),
            `${scenario.id}:${viewport} references a missing representative checkpoint`,
          );
          assert.ok(coverage.reason.trim().length > 0, `${scenario.id}:${viewport} needs a reason`);
        }
      }
    } else {
      assert.equal(scenario.role, "public", scenario.id);
      assert.deepEqual(capturedViewports, PUBLIC_VIEWPORTS, scenario.id);
    }
  }
});

test("scenario Course Appearance selection is stable and reports observed palette variety", () => {
  const first = SCREENSHOT_SCENARIOS.map((scenario) => scenarioCourseTheme(scenario.id));
  const second = SCREENSHOT_SCENARIOS.map((scenario) => scenarioCourseTheme(scenario.id));
  assert.deepEqual(second, first);

  const report = reportScenarioThemeVariety(SCREENSHOT_SCENARIOS);
  assert.match(report, /^Scenario Course Appearance palettes: \d+ of \d+ \(.+\)\.$/u);
});

test("PNG dimensions derive from image bytes", () => {
  const header = Buffer.alloc(24);
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]).copy(header);
  header.write("IHDR", 12, "ascii");
  header.writeUInt32BE(393, 16);
  header.writeUInt32BE(852, 20);
  assert.deepEqual(pngDimensions(header), { width: 393, height: 852 });
});

test("publication closes every declared Sysadmin laptop capture", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-sysadmin-publication-"));
  try {
    const manifest = await loadManifest(manifestPath);
    const sysadminCaptures = manifest.captures.filter((capture) => capture.role === "sysadmin");
    const sysadminPaths = sysadminCaptures.map((capture) => capture.path).sort();
    assert.equal(
      sysadminCaptures.every((capture) => capture.viewport === "laptop"),
      true,
    );

    await writeFixtureCorpus(root, manifest, "sysadmin-publication");
    const images = await inspectCorpus(root, manifest);
    const receipt = createReceipt("a".repeat(64), images);
    const atlas = renderAtlas(manifest, "screenshots/");
    const publishedSysadminPaths = images
      .filter((image) => image.path.startsWith("sysadmin/"))
      .map((image) => image.path);

    assert.deepEqual(publishedSysadminPaths, sysadminPaths);
    assert.deepEqual(
      receipt.paths.filter((capturePath) => capturePath.startsWith("sysadmin/")),
      sysadminPaths,
    );
    for (const sysadminPath of sysadminPaths) {
      assert.ok(atlas.includes(`screenshots/${sysadminPath}`), sysadminPath);
    }
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("duplicate PNG bytes remain valid and retain their receipt hashes", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-duplicate-bytes-"));
  try {
    const completeManifest = await loadManifest(manifestPath);
    const captures = completeManifest.captures
      .filter((capture) => capture.role === "instructor")
      .slice(0, 2);
    assert.equal(captures.length, 2);
    const manifest = { ...completeManifest, captures };
    await writeFixtureCorpus(root, manifest, "duplicate-bytes");
    const sharedBytes = fixturePng(captures[0], "identical-image");
    await Promise.all(
      captures.map((capture) => writeFile(path.join(root, capture.path), sharedBytes)),
    );

    const images = await inspectCorpus(root, manifest);
    const receipt = createReceipt("b".repeat(64), images);
    assert.equal(images.length, 2);
    assert.equal(images[0].sha256, images[1].sha256);
    assert.equal(receipt.images[0].sha256, receipt.images[1].sha256);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("eight generated folder galleries link their manifest images and appear in README", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-galleries-"));
  try {
    const manifest = await loadManifest(manifestPath);
    await writeScreenshotGalleries(root, manifest);
    await verifyScreenshotGalleries(path.join(root, "screenshot_galleries"), manifest);
    assert.equal(SCREENSHOT_GALLERY_FOLDERS.length, 8);
    const readme = await readFile(path.join(repositoryRoot, "README.md"), "utf8");
    for (const folder of SCREENSHOT_GALLERY_FOLDERS) {
      const page = await readFile(
        path.join(root, "screenshot_galleries", `${folder.id}.md`),
        "utf8",
      );
      const actualPaths = [...page.matchAll(/!\[[^\]]*\]\(\.\.\/screenshots\/([^)]+)\)/gu)].map(
        (match) => match[1],
      );
      const expectedPaths = manifest.captures
        .filter(
          (capture) =>
            capture.role === folder.role &&
            (folder.viewport === undefined || capture.viewport === folder.viewport),
        )
        .sort((left, right) => left.gallery.order - right.gallery.order)
        .map((capture) => capture.path);
      assert.deepEqual(actualPaths, expectedPaths, folder.id);
      assert.match(page, /\[Complete screenshot atlas\]\(\.\.\/SCREENSHOT_ATLAS\.md\)/u);
      assert.ok(
        readme.includes(`docs/screenshot_galleries/${folder.id}.md`),
        `README.md is missing ${folder.id}.md`,
      );
    }
  } finally {
    await rm(root, { force: true, recursive: true });
  }
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
    await writeFile(
      path.join(screenshotRoot, "coverage_exceptions.json"),
      `${JSON.stringify({ routes: [], ribbonDestinations: [] })}\n`,
      "utf8",
    );
    await writeFile(receiptPath, receiptJson(receipt), "utf8");
    await writeFile(atlasPath, renderAtlas(manifest, "screenshots/"), "utf8");
    await writeScreenshotGalleries(root, manifest);
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

test("replay publication requires the manifest that its receipt binds", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "ple-screenshot-replay-manifest-"));
  try {
    const manifest = oneCaptureManifest(await loadManifest(manifestPath));
    await writeFixtureCorpus(root, manifest, "replay");
    await assert.rejects(
      writeReplayArtifacts({ outputRoot: root, manifest, digest: "a".repeat(64) }),
      /screenshot corpus (?:required root file )?path set differs.*current_capture_manifest\.json/u,
    );
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
