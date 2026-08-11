#!/usr/bin/env node
// Capture the canonical real-stack walkthrough screenshots into docs/screenshots/.

import { spawn } from "node:child_process";
import { chmod, copyFile, lstat, mkdtemp, readdir, rename, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const screenshotNames = [
  "peptide_bond_mastery_overview.png",
  "student_fresh_practice.png",
  "instructor_gradebook_mastery_loop.png",
];
const privateTemporaryParent = "/private/tmp";
const privateTemporaryPrefix = "ple-docs-screenshots.";

function repositoryRoot() {
  const scriptPath = fileURLToPath(import.meta.url);
  const root = path.resolve(path.dirname(scriptPath), "../..");
  return root;
}

function walkthroughArguments() {
  const suppliedArguments = process.argv.slice(2);
  const prohibitedArguments = new Set([
    "--keep",
    "--instructor-setup-only",
    "--student-repeat-only",
  ]);
  for (const argument of suppliedArguments) {
    if (argument === "--skip-build") {
      throw new Error("--skip-build is not accepted; omit it for AUTO reuse or pass --build");
    }
    if (prohibitedArguments.has(argument)) {
      throw new Error("documentation screenshots require the full cleanup-enabled walkthrough");
    }
  }
  const hasMasterSeed = suppliedArguments.some(
    (argument) => argument === "--master-seed" || argument.startsWith("--master-seed="),
  );
  if (hasMasterSeed) return suppliedArguments;
  return ["--master-seed", "42", ...suppliedArguments];
}

async function validatePrivateDirectory(directory) {
  if (
    path.dirname(directory) !== privateTemporaryParent ||
    !path.basename(directory).startsWith(privateTemporaryPrefix)
  ) {
    throw new Error("documentation screenshot directory is outside the approved temporary path");
  }
  const metadata = await lstat(directory);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new Error("documentation screenshot directory must be a regular directory");
  }
  if (metadata.uid !== process.getuid() || (metadata.mode & 0o777) !== 0o700) {
    throw new Error("documentation screenshot directory ownership or mode is unsafe");
  }
}

async function validateCapturedScreenshots(directory) {
  await validatePrivateDirectory(directory);
  const entries = await readdir(directory, { withFileTypes: true });
  const names = entries.map((entry) => entry.name).sort();
  const expectedNames = [...screenshotNames].sort();
  if (
    names.length !== expectedNames.length ||
    names.some((name, index) => name !== expectedNames[index])
  ) {
    throw new Error("documentation capture did not produce exactly the expected PNG files");
  }
  for (const screenshotName of screenshotNames) {
    const screenshotPath = path.join(directory, screenshotName);
    const metadata = await lstat(screenshotPath);
    if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size === 0) {
      throw new Error("documentation capture produced an unsafe or empty PNG");
    }
  }
}

async function copyScreenshotsAtomically(repositoryRootPath, directory) {
  const destinationDirectory = path.join(repositoryRootPath, "docs", "screenshots");
  const destinationMetadata = await lstat(destinationDirectory);
  if (!destinationMetadata.isDirectory() || destinationMetadata.isSymbolicLink()) {
    throw new Error("docs/screenshots must be a regular directory");
  }
  for (const screenshotName of screenshotNames) {
    const sourcePath = path.join(directory, screenshotName);
    const destinationPath = path.join(destinationDirectory, screenshotName);
    const temporaryPath = path.join(
      destinationDirectory,
      `.${screenshotName}.${process.pid}.${Date.now().toString(36)}.tmp`,
    );
    await copyFile(sourcePath, temporaryPath);
    await chmod(temporaryPath, 0o644);
    await rename(temporaryPath, destinationPath);
  }
}

function runWalkthrough(repositoryRootPath, argumentsToPass, directory) {
  const childEnvironment = {
    ...process.env,
    PLE_DOCS_SCREENSHOT_DIR: directory,
  };
  return new Promise((resolve, reject) => {
    const child = spawn("bash", ["tests/walkthrough/run_ui_walkthrough.sh", ...argumentsToPass], {
      cwd: repositoryRootPath,
      env: childEnvironment,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      const detail = signal === null ? `exit status ${code}` : `signal ${signal}`;
      reject(new Error(`UI walkthrough failed with ${detail}`));
    });
  });
}

async function main() {
  const root = repositoryRoot();
  const argumentsToPass = walkthroughArguments();
  const directory = await mkdtemp(path.join(privateTemporaryParent, privateTemporaryPrefix));
  await chmod(directory, 0o700);
  try {
    await validatePrivateDirectory(directory);
    await runWalkthrough(root, argumentsToPass, directory);
    await validateCapturedScreenshots(directory);
    await copyScreenshotsAtomically(root, directory);
  } finally {
    await validatePrivateDirectory(directory);
    await rm(directory, { recursive: true, force: false });
  }
  process.stdout.write("PASS: documentation screenshots refreshed in docs/screenshots/.\n");
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown capture failure";
  process.stderr.write(`FAIL: documentation screenshot capture: ${message}\n`);
  process.exitCode = 1;
});
