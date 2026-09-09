// Manifest-driven screenshot corpus publisher and live replay verifier.

import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import { loadManifest, validateScenarioClosure, type CaptureManifest } from "./manifest";
import {
  manifestDigest,
  prepareOutputRoot,
  promoteCorpus,
  verifyPublishedArtifacts,
  writeReplayArtifacts,
  type PublishedImage,
} from "./publication";
import {
  createScenarioRuntime,
  loadSeededReferences,
  requireEntryUrl,
  requireProducedClosure,
} from "./runtime";
import { SCREENSHOT_SCENARIOS } from "./scenario_registry";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const screenshotRoot = path.join(repositoryRoot, "docs/screenshots");
const manifestPath = path.join(screenshotRoot, "current_capture_manifest.json");
const receiptPath = path.join(screenshotRoot, "current_capture_receipt.json");
const atlasPath = path.join(repositoryRoot, "docs/SCREENSHOT_ATLAS.md");
const resultRoot = path.join(repositoryRoot, "test-results/screenshot-corpus");
const reportPath = path.join(
  repositoryRoot,
  "local_stack_state/live_demo_browser/workspace/live_demo_course_report.json",
);

interface LoadedContract {
  readonly manifest: CaptureManifest;
  readonly digest: string;
}

interface PublishedContract extends LoadedContract {
  readonly images: ReadonlyArray<PublishedImage>;
}

async function loadContract(): Promise<LoadedContract> {
  const [manifest, digest] = await Promise.all([
    loadManifest(manifestPath),
    manifestDigest(manifestPath),
  ]);
  validateScenarioClosure(manifest, SCREENSHOT_SCENARIOS);
  return { manifest, digest };
}

async function verifyPublished(): Promise<PublishedContract> {
  const { manifest, digest } = await loadContract();
  const images = await verifyPublishedArtifacts({
    screenshotRoot,
    manifest,
    manifestDigest: digest,
    receiptPath,
    atlasPath,
  });
  return { manifest, digest, images };
}

async function replay(
  mode: "publish" | "verify",
  entryArgument: string | undefined,
): Promise<void> {
  const publishedBefore = mode === "verify" ? await verifyPublished() : undefined;
  const contract = publishedBefore ?? (await loadContract());
  const entryUrl = requireEntryUrl(entryArgument);
  const references = await loadSeededReferences(reportPath);
  const outputRoot = path.join(resultRoot, mode === "publish" ? "staging" : "verify");
  await prepareOutputRoot(outputRoot);
  const browser = await chromium.launch({ headless: true });
  try {
    const runtime = createScenarioRuntime({
      browser,
      entryUrl,
      outputRoot,
      manifest: contract.manifest,
      references,
    });
    for (const scenario of SCREENSHOT_SCENARIOS) {
      console.log(`Running screenshot scenario ${scenario.id}`);
      try {
        await scenario.run(runtime);
      } catch (error: unknown) {
        console.error(`Screenshot scenario ${scenario.id} failed.`);
        throw error;
      }
    }
    requireProducedClosure(runtime);
    if (mode === "publish") {
      await promoteCorpus({
        stagingRoot: outputRoot,
        screenshotRoot,
        atlasPath,
        manifest: contract.manifest,
        digest: contract.digest,
      });
      console.log("Published the complete screenshot corpus and generated atlas.");
      return;
    }
    const replayed = await writeReplayArtifacts({
      outputRoot,
      manifest: contract.manifest,
      digest: contract.digest,
    });
    const after = await verifyPublished();
    if (publishedBefore === undefined) throw new Error("verification lacks published artifacts");
    const beforeHashes = new Map(publishedBefore.images.map((image) => [image.path, image.sha256]));
    const afterHashes = new Map(after.images.map((image) => [image.path, image.sha256]));
    for (const [artifactPath, hash] of beforeHashes) {
      if (afterHashes.get(artifactPath) !== hash) {
        throw new Error(`live verification modified tracked screenshot ${artifactPath}`);
      }
    }
    const replayHashes = new Map(replayed.map((image) => [image.path, image.sha256]));
    const byteDifferenceCount = [...beforeHashes].filter(
      ([artifactPath, hash]) => replayHashes.get(artifactPath) !== hash,
    ).length;
    console.log(
      `Live replay passed; ${String(byteDifferenceCount)} tracked images differ byte-for-byte ` +
        "and remain available for human review in test-results/screenshot-corpus/verify/.",
    );
  } finally {
    await browser.close();
  }
}

export async function main(arguments_: ReadonlyArray<string>): Promise<void> {
  const [mode, entryArgument, ...extra] = arguments_;
  if (extra.length > 0) throw new Error("unexpected screenshot runner arguments");
  if (mode === "--verify-static" && entryArgument === undefined) {
    await verifyPublished();
    console.log("Static screenshot corpus verification passed.");
    return;
  }
  if (mode === "--publish") return replay("publish", entryArgument);
  if (mode === "--verify") return replay("verify", entryArgument);
  throw new Error("usage: capture_live_demo_screenshots.mjs --publish|--verify ENTRY_URL");
}
