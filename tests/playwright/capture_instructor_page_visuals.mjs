#!/usr/bin/env node
// Refresh the simulated instructor page gallery in docs/screenshots/.

import { spawn } from "node:child_process";
import { chmod, copyFile, lstat, mkdtemp, open, readdir, rename, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

import {
  CORPUS_VIEWPORT_SIZES,
  artifactNamesForPipeline,
  viewportForArtifact,
} from "./ui_corpus_manifest.ts";

// The manifest is the single authority for corpus membership, so this runner validates whatever the
// mock pipeline owns rather than carrying a second copy of the name list.
const screenshotNames = artifactNamesForPipeline("mock");
const temporaryParent = "/private/tmp";
const temporaryPrefix = "ple-docs-screenshots.";

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

function captureMode() {
  const argumentsToParse = process.argv.slice(2);
  if (argumentsToParse.length === 0) return "refresh";
  if (argumentsToParse.length === 1 && argumentsToParse[0] === "--verify-only") {
    return "verify";
  }
  throw new Error(
    "usage: node tests/playwright/capture_instructor_page_visuals.mjs [--verify-only]",
  );
}

async function validateDirectory(directory) {
  if (
    path.dirname(directory) !== temporaryParent ||
    !path.basename(directory).startsWith(temporaryPrefix)
  ) {
    throw new Error("instructor screenshot directory is outside the approved temporary path");
  }
  const metadata = await lstat(directory);
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    metadata.uid !== process.getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error("instructor screenshot directory ownership or mode is unsafe");
  }
}

async function pngDimensions(filePath) {
  const handle = await open(filePath, "r");
  try {
    const header = Buffer.alloc(24);
    const { bytesRead } = await handle.read(header, 0, header.length, 0);
    if (
      bytesRead !== header.length ||
      !header.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
    ) {
      throw new Error("capture is not a valid PNG");
    }
    return { width: header.readUInt32BE(16), height: header.readUInt32BE(20) };
  } finally {
    await handle.close();
  }
}

async function validateScreenshots(directory) {
  await validateDirectory(directory);
  const entries = await readdir(directory, { withFileTypes: true });
  const names = entries.map((entry) => entry.name).sort();
  const expected = [...screenshotNames].sort();
  if (names.length !== expected.length || names.some((name, index) => name !== expected[index])) {
    throw new Error("instructor capture did not produce exactly the expected PNG files");
  }
  for (const name of screenshotNames) {
    const filePath = path.join(directory, name);
    const metadata = await lstat(filePath);
    if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size === 0) {
      throw new Error(`${name} is not a nonempty regular file`);
    }
    const viewport = viewportForArtifact(name);
    if (viewport === undefined) {
      throw new Error(`${name} needs a viewport declared in the corpus manifest`);
    }
    const expected = CORPUS_VIEWPORT_SIZES[viewport];
    const dimensions = await pngDimensions(filePath);
    if (dimensions.width !== expected.width || dimensions.height !== expected.height) {
      throw new Error(
        `${name} must be exactly ${expected.width} by ${expected.height} CSS pixels for the ${viewport} viewport`,
      );
    }
  }
}

function runCapture(root, directory) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "npx",
      ["playwright", "test", "tests/playwright/instructor_page_visuals.spec.ts", "--workers=1"],
      {
        cwd: root,
        env: { ...process.env, PLE_INSTRUCTOR_PAGE_VISUALS_DIR: directory },
        stdio: "inherit",
      },
    );
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve();
      else
        reject(
          new Error(signal === null ? `Playwright exited ${code}` : `Playwright got ${signal}`),
        );
    });
  });
}

async function copyScreenshots(root, directory) {
  const destination = path.join(root, "docs", "screenshots");
  const destinationMetadata = await lstat(destination);
  if (!destinationMetadata.isDirectory() || destinationMetadata.isSymbolicLink()) {
    throw new Error("docs/screenshots must be a regular directory");
  }
  for (const name of screenshotNames) {
    const temporary = path.join(destination, `.${name}.${process.pid}.tmp`);
    await copyFile(path.join(directory, name), temporary);
    await chmod(temporary, 0o644);
    await rename(temporary, path.join(destination, name));
  }
}

async function main() {
  const root = repositoryRoot();
  const mode = captureMode();
  const directory = await mkdtemp(path.join(temporaryParent, temporaryPrefix));
  await chmod(directory, 0o700);
  try {
    await validateDirectory(directory);
    await runCapture(root, directory);
    await validateScreenshots(directory);
    if (mode === "refresh") await copyScreenshots(root, directory);
  } finally {
    await validateDirectory(directory);
    await rm(directory, { recursive: true, force: false });
  }
  const result = mode === "refresh" ? "refreshed" : "verified without changing docs";
  process.stdout.write(`PASS: simulated instructor page visuals ${result}.\n`);
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown capture failure";
  process.stderr.write(`FAIL: simulated instructor page visuals: ${message}\n`);
  process.exitCode = 1;
});
