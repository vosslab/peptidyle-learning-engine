// Manifest-driven screenshot corpus publisher and live replay verifier.

import path from "node:path";
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "../helper_gateway_trust.mjs";

import {
  assertManifestMatches,
  generateManifest,
  loadCoverageExceptions,
  loadManifest,
  manifestJson,
  type CaptureManifest,
  type CaptureRecord,
} from "./manifest";
import {
  manifestDigest,
  prepareOutputRoot,
  promoteCorpus,
  verifyPublishedArtifacts,
  writeReplayArtifacts,
  type PublishedImage,
} from "./publication";
import { createScenarioRuntime, requireEntryUrl, requireProducedClosure } from "./runtime";
import { SCREENSHOT_SCENARIOS } from "./scenario_registry";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const screenshotRoot = path.join(repositoryRoot, "docs/screenshots");
const manifestPath = path.join(screenshotRoot, "current_capture_manifest.json");
const exceptionsPath = path.join(screenshotRoot, "coverage_exceptions.json");
const receiptPath = path.join(screenshotRoot, "current_capture_receipt.json");
const atlasPath = path.join(repositoryRoot, "docs/SCREENSHOT_ATLAS.md");
const resultRoot = path.join(repositoryRoot, "test-results/screenshot-corpus");

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
  mode: "publish" | "verify" | "only",
  entryArgument: string | undefined,
  headed: boolean,
  onlyScenarioIds: ReadonlySet<string> = new Set(),
): Promise<void> {
  const publishedBefore = mode === "verify" ? await verifyPublished() : undefined;
  const exceptions = await loadCoverageExceptions(exceptionsPath);
  const entryUrl = requireEntryUrl(entryArgument);
  const outputRoot = path.join(resultRoot, mode === "verify" ? "verify" : "staging");
  await prepareOutputRoot(outputRoot);
  const browser = await chromium.launch({
    headless: !headed,
    args: liveDemoChromiumArgs(entryUrl.href),
  });
  const produced: CaptureRecord[] = [];
  try {
    const selected = SCREENSHOT_SCENARIOS.filter(
      (scenario) => mode !== "only" || onlyScenarioIds.has(scenario.id),
    );
    if (mode === "only" && selected.length !== onlyScenarioIds.size) {
      throw new Error("unknown screenshot scenario id in --only selection");
    }
    for (const scenario of selected) {
      console.log(`Running screenshot scenario ${scenario.id}`);
      const runtime = createScenarioRuntime({
        browser,
        entryUrl,
        outputRoot,
        scenario,
      });
      try {
        await scenario.run(runtime);
      } catch (error: unknown) {
        console.error(`Screenshot scenario ${scenario.id} failed.`);
        throw error;
      }
      requireProducedClosure(runtime, scenario);
      produced.push(...runtime.producedCaptures);
    }
    if (mode === "only") {
      console.log(`Ran ${String(selected.length)} selected scenario(s) into ${outputRoot}.`);
      return;
    }
    const generated = generateManifest(produced, exceptions);
    const encoded = manifestJson(generated);
    await writeFile(path.join(outputRoot, "current_capture_manifest.json"), encoded, "utf8");
    const digest = await manifestDigest(path.join(outputRoot, "current_capture_manifest.json"));
    if (mode === "publish") {
      await promoteCorpus({
        stagingRoot: outputRoot,
        screenshotRoot,
        atlasPath,
        manifest: generated,
        digest,
      });
      console.log("Published the complete screenshot corpus and generated atlas.");
      return;
    }
    if (publishedBefore === undefined) throw new Error("verification lacks published artifacts");
    assertManifestMatches(publishedBefore.manifest, generated);
    const replayed = await writeReplayArtifacts({
      outputRoot,
      manifest: generated,
      digest,
    });
    const after = await verifyPublished();
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
  const [mode, firstArgument, secondArgument, ...extra] = arguments_;
  if (mode === "--verify-static" && firstArgument === undefined) {
    await verifyPublished();
    console.log("Static screenshot corpus verification passed.");
    return;
  }
  if (mode === "--only") {
    if (firstArgument === undefined || secondArgument === undefined || extra.length > 0) {
      throw new Error("usage: capture_live_demo_screenshots.mjs --only ENTRY_URL id[,id...]");
    }
    return replay("only", firstArgument, false, new Set(secondArgument.split(",")));
  }
  const headed = firstArgument === "--headed";
  const entryArgument = headed ? secondArgument : firstArgument;
  if (extra.length > 0 || (headed ? secondArgument === undefined : secondArgument !== undefined)) {
    throw new Error("unexpected screenshot runner arguments");
  }
  if (mode === "--publish") return replay("publish", entryArgument, headed);
  if (mode === "--verify") return replay("verify", entryArgument, headed);
  throw new Error(
    "usage: capture_live_demo_screenshots.mjs --publish|--verify [--headed] ENTRY_URL " +
      "| --only ENTRY_URL id[,id...]",
  );
}
