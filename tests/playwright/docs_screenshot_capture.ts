/** Opt-in documentation screenshot capture for the real UI walkthrough. */

import { chmod, lstat, mkdir, open } from "node:fs/promises";
import path from "node:path";

import type { Locator, Page } from "@playwright/test";

import {
  artifactPathForBasename,
  captureOwnerForArtifact,
  CORPUS_DIRECTORY,
  CORPUS_VIEWPORT_SIZES,
  manifestArtifactPaths,
  viewportForArtifact,
  type CorpusCaptureOwner,
} from "./ui_corpus_manifest";

const CAPTURE_DIRECTORY_PARENT = "/private/tmp";
const CAPTURE_DIRECTORY_PREFIX = "ple-docs-screenshots.";
const CAPTURE_OWNER_ENVIRONMENT = "PLE_UI_CORPUS_CAPTURE_OWNER";

export const documentationScreenshotNames = manifestArtifactPaths().map((artifactPath) =>
  path.posix.basename(artifactPath),
);

export type DocumentationScreenshotName = string;

export function documentationScreenshotsEnabled(screenshotDirectory?: string): boolean {
  return screenshotDirectory !== undefined;
}

function isMissingPathError(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}

async function validateCaptureDirectory(directory: string): Promise<void> {
  if (!path.isAbsolute(directory)) {
    throw new Error("documentation screenshot directory must be absolute");
  }
  if (
    path.dirname(directory) !== CAPTURE_DIRECTORY_PARENT ||
    !path.basename(directory).startsWith(CAPTURE_DIRECTORY_PREFIX)
  ) {
    throw new Error("documentation screenshot directory must be runner-created under /private/tmp");
  }
  const metadata = await lstat(directory);
  const getuid = process.getuid;
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    getuid === undefined ||
    metadata.uid !== getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error("documentation screenshot directory ownership or mode is unsafe");
  }
}

async function validatePrivateChildDirectory(directory: string): Promise<void> {
  const metadata = await lstat(directory);
  const getuid = process.getuid;
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    getuid === undefined ||
    metadata.uid !== getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error(`documentation screenshot child directory is unsafe: ${directory}`);
  }
}

async function ensureCaptureParent(
  screenshotDirectory: string,
  artifactPath: string,
): Promise<string> {
  const relativePath = path.posix.relative(CORPUS_DIRECTORY, artifactPath);
  if (relativePath.startsWith("../") || path.posix.isAbsolute(relativePath)) {
    throw new Error("documentation screenshot artifact escapes the corpus directory");
  }
  const relativeParent = path.posix.dirname(relativePath);
  let current = screenshotDirectory;
  if (relativeParent !== ".") {
    for (const segment of relativeParent.split("/")) {
      current = path.join(current, segment);
      try {
        await mkdir(current, { mode: 0o700 });
      } catch (error: unknown) {
        if (!isExistingPathError(error)) throw error;
      }
      await validatePrivateChildDirectory(current);
    }
  }
  const target = path.join(screenshotDirectory, ...relativePath.split("/"));
  const resolvedRoot = `${path.resolve(screenshotDirectory)}${path.sep}`;
  if (!path.resolve(target).startsWith(resolvedRoot)) {
    throw new Error("documentation screenshot target escapes the private directory");
  }
  return target;
}

function isExistingPathError(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === "EEXIST";
}

async function validateScreenshotPath(filePath: string): Promise<void> {
  try {
    const metadata = await lstat(filePath);
    if (metadata.isSymbolicLink() || !metadata.isFile()) {
      throw new Error("documentation screenshot path must be a regular file");
    }
    throw new Error("documentation screenshot path already exists");
  } catch (error: unknown) {
    if (isMissingPathError(error)) return;
    throw error;
  }
}

function selectedCaptureOwner(): CorpusCaptureOwner | undefined {
  const value = process.env[CAPTURE_OWNER_ENVIRONMENT];
  if (value === undefined) return undefined;
  if (
    value === "instructorMock" ||
    value === "studentMock" ||
    value === "t2Mock" ||
    value === "live"
  ) {
    return value;
  }
  throw new Error(`${CAPTURE_OWNER_ENVIRONMENT} names an unknown capture owner`);
}

async function pngDimensions(filePath: string): Promise<{ width: number; height: number }> {
  const handle = await open(filePath, "r");
  try {
    const header = Buffer.alloc(24);
    const { bytesRead } = await handle.read(header, 0, header.length, 0);
    if (
      bytesRead !== header.length ||
      !header.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
    ) {
      throw new Error("documentation screenshot is not a valid PNG");
    }
    return { width: header.readUInt32BE(16), height: header.readUInt32BE(20) };
  } finally {
    await handle.close();
  }
}

/** Capture one bounded public UI state only when the dedicated launcher opts in. */
export async function captureDocumentationScreenshot(
  page: Page,
  screenshotName: DocumentationScreenshotName,
  anchor?: Locator,
  cropTopPixels?: number,
  screenshotDirectory?: string,
  anchorAlignment: "top" | "bottom" = "top",
): Promise<void> {
  if (screenshotDirectory === undefined) return;
  const artifactPath = artifactPathForBasename(screenshotName);
  if (artifactPath === undefined) {
    throw new Error("documentation screenshot name is not approved");
  }
  const owner = selectedCaptureOwner();
  if (owner !== undefined && captureOwnerForArtifact(artifactPath) !== owner) return;
  const viewport = viewportForArtifact(artifactPath);
  if (viewport === undefined) {
    throw new Error("documentation screenshot needs a manifest viewport");
  }
  const viewportSize = CORPUS_VIEWPORT_SIZES[viewport];
  if (
    cropTopPixels !== undefined &&
    (!Number.isInteger(cropTopPixels) || cropTopPixels < 0 || cropTopPixels >= viewportSize.height)
  ) {
    throw new Error("documentation screenshot crop must be below the viewport height");
  }
  await validateCaptureDirectory(screenshotDirectory);
  const filePath = await ensureCaptureParent(screenshotDirectory, artifactPath);
  await validateScreenshotPath(filePath);
  await page.setViewportSize(viewportSize);
  if (anchor !== undefined) {
    await anchor.scrollIntoViewIfNeeded();
    const anchorMargin = cropTopPixels === undefined ? 72 : 0;
    await anchor.evaluate(
      (element, options) => {
        const bounds = element.getBoundingClientRect();
        const target =
          options.alignment === "bottom"
            ? bounds.bottom - window.innerHeight + options.margin
            : bounds.top - options.margin;
        window.scrollBy(0, target);
      },
      { alignment: anchorAlignment, margin: anchorMargin },
    );
  }
  const devicePixelRatio = await page.evaluate(() => window.devicePixelRatio);
  if (devicePixelRatio !== 1) {
    throw new Error("documentation screenshots require device pixel ratio 1");
  }
  await page.screenshot({
    path: filePath,
    animations: "disabled",
    caret: "hide",
    scale: "css",
  });
  const metadata = await lstat(filePath);
  if (metadata.isSymbolicLink() || !metadata.isFile() || metadata.size === 0) {
    throw new Error("documentation screenshot was not written as a nonempty regular file");
  }
  const dimensions = await pngDimensions(filePath);
  if (dimensions.width !== viewportSize.width || dimensions.height !== viewportSize.height) {
    throw new Error(
      `${artifactPath} must be exactly ${viewportSize.width} by ${viewportSize.height} CSS pixels`,
    );
  }
  await chmod(filePath, 0o644);
}
