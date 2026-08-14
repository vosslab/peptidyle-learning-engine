#!/usr/bin/env node
// Verify course-appearance visual evidence without changing retained artifacts.

import { spawn } from "node:child_process";
import { chmod, lstat, mkdtemp, open, readdir, readFile, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const temporaryParent = "/private/tmp";
const temporaryPrefix = "ple-course-appearance-visuals.";
const expectedArtifacts = [
  "palette_metrics.json",
  "settings_1280x800.png",
  "settings_forced_colors.png",
  "theme_contact_sheet.png",
];

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

async function validateDirectory(directory) {
  if (
    path.dirname(directory) !== temporaryParent ||
    !path.basename(directory).startsWith(temporaryPrefix)
  ) {
    throw new Error("course-appearance visual directory is outside the approved temporary path");
  }
  const metadata = await lstat(directory);
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    metadata.uid !== process.getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error("course-appearance visual directory ownership or mode is unsafe");
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
      throw new Error(`${path.basename(filePath)} is not a valid PNG`);
    }
    return { width: header.readUInt32BE(16), height: header.readUInt32BE(20) };
  } finally {
    await handle.close();
  }
}

async function validatePng(directory, name, minimumWidth, minimumHeight) {
  const filePath = path.join(directory, name);
  const metadata = await lstat(filePath);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size === 0) {
    throw new Error(`${name} is not a nonempty regular file`);
  }
  const dimensions = await pngDimensions(filePath);
  if (dimensions.width < minimumWidth || dimensions.height < minimumHeight) {
    throw new Error(
      `${name} is ${dimensions.width} by ${dimensions.height}; expected at least ${minimumWidth} by ${minimumHeight}`,
    );
  }
}

async function validateMetrics(directory) {
  const metricsPath = path.join(directory, "palette_metrics.json");
  const metadata = await lstat(metricsPath);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size === 0) {
    throw new Error("palette_metrics.json is not a nonempty regular file");
  }
  const document = JSON.parse(await readFile(metricsPath, "utf8"));
  if (
    document.formatVersion !== 1 ||
    document.thresholds?.normalTextContrast !== 5.5 ||
    !Array.isArray(document.renderedThemes) ||
    document.renderedThemes.length === 0 ||
    !Array.isArray(document.oklabDedup)
  ) {
    throw new Error("palette_metrics.json does not contain the expected visual evidence");
  }
}

async function validateArtifacts(directory) {
  await validateDirectory(directory);
  const entries = await readdir(directory, { withFileTypes: true });
  const names = entries.map((entry) => entry.name).sort();
  const expected = [...expectedArtifacts].sort();
  if (names.length !== expected.length || names.some((name, index) => name !== expected[index])) {
    throw new Error(
      `course-appearance capture did not produce the expected artifacts: ${names.join(", ")}`,
    );
  }
  await validateMetrics(directory);
  await validatePng(directory, "settings_1280x800.png", 1_280, 800);
  await validatePng(directory, "settings_forced_colors.png", 1_280, 800);
  await validatePng(directory, "theme_contact_sheet.png", 1_200, 800);
}

function runCapture(root, directory) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "npx",
      ["playwright", "test", "tests/playwright/course_appearance_visual.spec.ts", "--workers=1"],
      {
        cwd: root,
        env: {
          ...process.env,
          PLE_CAPTURE_COURSE_APPEARANCE_VISUALS: "1",
          PLE_COURSE_APPEARANCE_VISUALS_DIR: directory,
        },
        stdio: "inherit",
      },
    );
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve();
      else {
        const detail = signal === null ? `exit status ${code}` : `signal ${signal}`;
        reject(new Error(`Playwright course-appearance visual capture failed with ${detail}`));
      }
    });
  });
}

async function main() {
  const root = repositoryRoot();
  const directory = await mkdtemp(path.join(temporaryParent, temporaryPrefix));
  await chmod(directory, 0o700);
  try {
    await validateDirectory(directory);
    await runCapture(root, directory);
    await validateArtifacts(directory);
  } finally {
    await validateDirectory(directory);
    await rm(directory, { recursive: true, force: false });
  }
  process.stdout.write(
    "PASS: course-appearance visuals verified without changing retained artifacts.\n",
  );
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown verification failure";
  process.stderr.write(`FAIL: course-appearance visuals: ${message}\n`);
  process.exitCode = 1;
});
