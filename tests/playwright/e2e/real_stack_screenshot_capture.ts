// Closed real-stack capture hook. It is inert for ordinary focused journeys.
import { createHash } from "node:crypto";
import {
  closeSync,
  constants,
  fstatSync,
  fsyncSync,
  lstatSync,
  openSync,
  writeSync,
} from "node:fs";
import path from "node:path";

import type { Stats } from "node:fs";

import type { Page } from "@playwright/test";

import type { BrowserScenarioInputV1 } from "../browser_suite_live_config";
import {
  CORPUS_VIEWPORT_SIZES,
  UI_CORPUS_MANIFEST,
  type CorpusArtifact,
} from "../ui_corpus_manifest";

interface DirectoryIdentity {
  readonly device: number;
  readonly inode: number;
  readonly uid: number;
  readonly mode: number;
}

interface StagingGuard {
  readonly path: string;
  readonly descriptor: number;
  readonly identity: DirectoryIdentity;
}

const privateTextPatterns: readonly RegExp[] = [
  /(?:^|[\s"'(])\/(?:Users|private|tmp|var|workspace|repo|repository)(?:\/|\b)/iu,
  /\b[A-Z]:\\(?:Users|Temp|Windows|workspace|repo)(?:\\|\b)/iu,
  /\bPLE_BROWSER(?:_[A-Z0-9]+)*\b/u,
  /\b(?:authorization|cookie)\s*:/iu,
  /\bbearer\s+[A-Za-z0-9._~+/-]+=*/iu,
  /\b(?:request|response)\s+headers?\b/iu,
  /\bownership proof\b/iu,
  /\b(?:private|owner|browser|suite|disposable)\s+(?:capability|manifest)\b/iu,
  /\b(?:capability|manifest)\s+(?:file|path|token|secret|value)\b/iu,
  /\binvitation\s+(?:link|token)\b/iu,
  /\bdiagnostic overlay\b/iu,
];
const emailPattern = /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/iu;

function declaredArtifact(
  input: BrowserScenarioInputV1,
  artifactId: string,
): CorpusArtifact | undefined {
  const capture = input.screenshotCapture;
  if (capture === undefined) return undefined;
  const expected = UI_CORPUS_MANIFEST.filter(
    (candidate) => candidate.scenarioId === input.scenarioId,
  );
  const requestedKeys = capture.artifacts
    .map((candidate) => `${candidate.artifactId}:${candidate.stateId}`)
    .sort();
  const expectedKeys = expected
    .map((candidate) => `${candidate.artifactId}:${candidate.stateId}`)
    .sort();
  if (
    requestedKeys.length !== expectedKeys.length ||
    requestedKeys.some((candidate, index) => candidate !== expectedKeys[index])
  ) {
    throw new Error("screenshot capture artifacts are not the closed scenario declaration");
  }
  const requested = capture.artifacts.find((candidate) => candidate.artifactId === artifactId);
  if (requested === undefined)
    throw new Error("screenshot capture artifact is not declared by this scenario");
  const artifact = UI_CORPUS_MANIFEST.find(
    (candidate) =>
      candidate.artifactId === requested.artifactId &&
      candidate.stateId === requested.stateId &&
      candidate.scenarioId === input.scenarioId,
  );
  if (artifact === undefined)
    throw new Error("screenshot capture artifact is not declared by this scenario");
  return artifact;
}

function directoryIdentity(metadata: Stats): DirectoryIdentity {
  return {
    device: metadata.dev,
    inode: metadata.ino,
    uid: metadata.uid,
    mode: metadata.mode & 0o777,
  };
}

function sameDirectoryIdentity(left: DirectoryIdentity, right: DirectoryIdentity): boolean {
  return (
    left.device === right.device &&
    left.inode === right.inode &&
    left.uid === right.uid &&
    left.mode === right.mode
  );
}

function requireSafeDirectory(metadata: Stats, uid: number): void {
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    metadata.uid !== uid ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error("screenshot staging is unsafe");
  }
}

function privateStagingFromEnvironment(): StagingGuard {
  const staging = process.env.PLE_BROWSER_SUITE_SCREENSHOT_STAGING;
  if (staging === undefined || staging === "")
    throw new Error("owner did not provide screenshot staging");
  if (!path.isAbsolute(staging) || path.resolve(staging) !== staging) {
    throw new Error("screenshot staging must be an absolute canonical directory");
  }
  const uid = process.getuid?.();
  if (uid === undefined) throw new Error("screenshot staging is unsafe");
  const metadata = lstatSync(staging);
  requireSafeDirectory(metadata, uid);
  const identity = directoryIdentity(metadata);
  const descriptor = openSync(
    staging,
    constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW,
  );
  try {
    const held = fstatSync(descriptor);
    requireSafeDirectory(held, uid);
    if (!sameDirectoryIdentity(identity, directoryIdentity(held))) {
      throw new Error("screenshot staging changed while it was opened");
    }
  } catch (error) {
    closeSync(descriptor);
    throw error;
  }
  return { path: staging, descriptor, identity };
}

function assertGuardStable(guard: StagingGuard): void {
  const latest = lstatSync(guard.path);
  const held = fstatSync(guard.descriptor);
  const uid = process.getuid?.();
  if (uid === undefined) throw new Error("screenshot staging is unsafe");
  requireSafeDirectory(latest, uid);
  requireSafeDirectory(held, uid);
  if (
    !sameDirectoryIdentity(guard.identity, directoryIdentity(latest)) ||
    !sameDirectoryIdentity(guard.identity, directoryIdentity(held))
  ) {
    throw new Error("screenshot staging changed during capture");
  }
}

function writePrivateFile(guard: StagingGuard, filename: string, content: Buffer): void {
  // Node has no openat binding. This held-fd identity lease detects same-UID path replacement
  // before open, after open, and before bytes are written; the final pathname relookup race is
  // deliberately narrow and remains owned by the private, suite-created 0700 staging boundary.
  assertGuardStable(guard);
  const target = path.join(guard.path, filename);
  const descriptor = openSync(
    target,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600,
  );
  try {
    assertGuardStable(guard);
    const child = fstatSync(descriptor);
    const uid = process.getuid?.();
    if (
      uid === undefined ||
      !child.isFile() ||
      child.uid !== uid ||
      (child.mode & 0o777) !== 0o600
    ) {
      throw new Error("private screenshot artifact is unsafe");
    }
    assertGuardStable(guard);
    let offset = 0;
    while (offset < content.length) {
      const written = writeSync(descriptor, content, offset, content.length - offset);
      if (written <= 0) throw new Error("private screenshot artifact write did not advance");
      offset += written;
    }
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

export function validateVisibleScreenshotText(visible: string, permitsMaskedEmail = false): void {
  for (const pattern of privateTextPatterns) {
    if (pattern.test(visible))
      throw new Error("screenshot state exposes private diagnostic material");
  }
  if (!permitsMaskedEmail && emailPattern.test(visible)) {
    throw new Error("screenshot state exposes an unmasked email address");
  }
}

async function maskVisibleEmailElements(page: Page): Promise<void> {
  const masked = await page.locator("body *").evaluateAll((elements) => {
    const email = /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/iu;
    let count = 0;
    for (const element of elements) {
      const style = window.getComputedStyle(element);
      if (
        style.display === "none" ||
        style.visibility === "hidden" ||
        !(element instanceof HTMLElement) ||
        element.getClientRects().length === 0
      ) {
        continue;
      }
      const directText = Array.from(element.childNodes)
        .filter((node) => node.nodeType === Node.TEXT_NODE)
        .map((node) => node.textContent ?? "")
        .join(" ");
      const inputValue =
        element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement
          ? element.value
          : "";
      if (!email.test(directText) && !email.test(inputValue)) continue;
      element.style.setProperty("background-color", "#d1d5db", "important");
      element.style.setProperty("color", "#d1d5db", "important");
      element.style.setProperty("text-shadow", "none", "important");
      count += 1;
    }
    return count;
  });
  if (masked === 0) throw new Error("email-masked screenshot state has no visible email element");
}

function requireKnownPrivacyChecks(artifact: CorpusArtifact): void {
  const known = new Set(["no_private_material", "no_feedback", "email_masked"]);
  if (
    artifact.privacyChecks.length === 0 ||
    artifact.privacyChecks.some((check) => !known.has(check))
  ) {
    throw new Error("screenshot artifact has an unsupported privacy check");
  }
}

async function validateVisibleCaptureState(page: Page, artifact: CorpusArtifact): Promise<void> {
  requireKnownPrivacyChecks(artifact);
  const masksEmail = artifact.privacyChecks.includes("email_masked");
  if (masksEmail) await maskVisibleEmailElements(page);
  if (artifact.privacyChecks.includes("no_private_material")) {
    validateVisibleScreenshotText(await page.locator("body").innerText(), masksEmail);
  }
  if (
    artifact.privacyChecks.includes("no_feedback") &&
    (await page.locator(".feedback-panel").count()) !== 0
  ) {
    throw new Error("pre-feedback screenshot state exposes an evaluation surface");
  }
}

function validateCaptureRequest(
  page: Page,
  input: BrowserScenarioInputV1,
  artifact: CorpusArtifact,
): string {
  const viewport = CORPUS_VIEWPORT_SIZES[artifact.viewport];
  const actual = page.viewportSize();
  if (actual?.width !== viewport.width || actual.height !== viewport.height) {
    throw new Error("screenshot capture viewport does not match the manifest profile");
  }
  const expectedOrigin = new URL(input.baseUrl).origin;
  if (new URL(page.url()).origin !== expectedOrigin)
    throw new Error("screenshot capture origin is invalid");
  if (artifact.scenarioId !== input.scenarioId) {
    throw new Error("screenshot capture artifact belongs to another scenario");
  }
  return expectedOrigin;
}

function captureReceipt(
  artifact: CorpusArtifact,
  input: BrowserScenarioInputV1,
  digest: string,
  origin: string,
  width: number,
  height: number,
): Buffer {
  const receipt = {
    artifactId: artifact.artifactId,
    scenarioId: input.scenarioId,
    stateId: artifact.stateId,
    sha256: digest,
    viewport: artifact.viewport,
    width,
    height,
    origin,
    privacyValidated: true,
    privacyChecks: artifact.privacyChecks,
  };
  return Buffer.from(JSON.stringify(receipt), "ascii");
}

async function stabilizeCapturePage(page: Page): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addStyleTag({
    content:
      "*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important}",
  });
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
  });
}

async function applyArtifactViewport(page: Page, artifact: CorpusArtifact): Promise<void> {
  const viewport = CORPUS_VIEWPORT_SIZES[artifact.viewport];
  await page.setViewportSize(viewport);
  const actual = page.viewportSize();
  if (actual?.width !== viewport.width || actual.height !== viewport.height) {
    throw new Error("screenshot capture could not apply the manifest viewport profile");
  }
}

async function restoreCanonicalViewport(page: Page): Promise<void> {
  await page.setViewportSize(CORPUS_VIEWPORT_SIZES.laptop);
}

/** Capture a declared UI state to owner-selected private staging, if requested. */
export async function captureRealStackScreenshot(
  page: Page,
  input: BrowserScenarioInputV1,
  artifactId: string,
): Promise<void> {
  const artifact = declaredArtifact(input, artifactId);
  if (artifact === undefined) return;
  try {
    await applyArtifactViewport(page, artifact);
    const origin = validateCaptureRequest(page, input, artifact);
    const deviceScaleFactor = await page.evaluate(() => window.devicePixelRatio);
    const viewport = CORPUS_VIEWPORT_SIZES[artifact.viewport];
    if (deviceScaleFactor !== viewport.deviceScaleFactor) {
      throw new Error("screenshot capture device scale factor does not match the manifest profile");
    }
    await stabilizeCapturePage(page);
    await validateVisibleCaptureState(page, artifact);
    const png = await page.screenshot({ animations: "disabled", caret: "hide" });
    const digest = createHash("sha256").update(png).digest("hex");
    const actualViewport = page.viewportSize();
    if (actualViewport === null) {
      throw new Error("screenshot capture lost its declared viewport");
    }
    const staging = privateStagingFromEnvironment();
    try {
      writePrivateFile(staging, `${artifact.artifactId}.png`, png);
      writePrivateFile(
        staging,
        `${artifact.artifactId}.json`,
        captureReceipt(
          artifact,
          input,
          digest,
          origin,
          actualViewport.width,
          actualViewport.height,
        ),
      );
    } finally {
      closeSync(staging.descriptor);
    }
  } finally {
    await restoreCanonicalViewport(page);
  }
}
