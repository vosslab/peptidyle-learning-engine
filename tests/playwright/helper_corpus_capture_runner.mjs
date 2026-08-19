// Shared private-directory, validation, atomic-copy, cleanup, and provenance lifecycle.

import { constants as fsConstants } from "node:fs";
import {
  chmod,
  copyFile,
  lstat,
  mkdir,
  mkdtemp,
  open,
  readdir,
  rename,
  rm,
} from "node:fs/promises";
import path from "node:path";

import {
  artifactPathsForCaptureOwner,
  CORPUS_DIRECTORY,
  CORPUS_VIEWPORT_SIZES,
  surfaceForArtifact,
  viewportForArtifact,
} from "./ui_corpus_manifest.ts";
import { recordProvenance } from "./ui_corpus_provenance.mjs";

const PRIVATE_TEMPORARY_PARENT = "/private/tmp";
const PRIVATE_TEMPORARY_PREFIX = "ple-docs-screenshots.";

function isPathError(error, code) {
  return typeof error === "object" && error !== null && "code" in error && error.code === code;
}

async function validatePrivateDirectory(directory, root = false) {
  if (
    root &&
    (path.dirname(directory) !== PRIVATE_TEMPORARY_PARENT ||
      !path.basename(directory).startsWith(PRIVATE_TEMPORARY_PREFIX))
  ) {
    throw new Error("corpus capture directory is outside the approved temporary path");
  }
  const metadata = await lstat(directory);
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    metadata.uid !== process.getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error(`corpus capture directory ownership or mode is unsafe: ${directory}`);
  }
}

function temporaryRelativePath(artifactPath) {
  const relativePath = path.posix.relative(CORPUS_DIRECTORY, artifactPath);
  if (relativePath.startsWith("../") || path.posix.isAbsolute(relativePath)) {
    throw new Error(`artifact escapes the corpus directory: ${artifactPath}`);
  }
  return relativePath;
}

async function collectCaptureFiles(directory, relativeDirectory = "") {
  await validatePrivateDirectory(directory, relativeDirectory === "");
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const relativePath =
      relativeDirectory === "" ? entry.name : `${relativeDirectory}/${entry.name}`;
    const target = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error(`capture contains a symbolic link: ${relativePath}`);
    }
    if (entry.isDirectory()) {
      files.push(...(await collectCaptureFiles(target, relativePath)));
      continue;
    }
    if (!entry.isFile()) {
      throw new Error(`capture contains an unsupported entry: ${relativePath}`);
    }
    files.push(relativePath);
  }
  return files;
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
      throw new Error(`${filePath} is not a valid PNG`);
    }
    return { width: header.readUInt32BE(16), height: header.readUInt32BE(20) };
  } finally {
    await handle.close();
  }
}

async function validateCapturedScreenshots(directory, artifactPaths) {
  const actual = (await collectCaptureFiles(directory)).sort();
  const expected = artifactPaths.map(temporaryRelativePath).sort();
  if (actual.length !== expected.length || actual.some((name, index) => name !== expected[index])) {
    throw new Error("capture did not produce exactly the manifest-owned nested PNG paths");
  }
  for (const artifactPath of artifactPaths) {
    const screenshotPath = path.join(directory, ...temporaryRelativePath(artifactPath).split("/"));
    const metadata = await lstat(screenshotPath);
    if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size === 0) {
      throw new Error(`${artifactPath} is not a nonempty regular file`);
    }
    const viewport = viewportForArtifact(artifactPath);
    if (viewport === undefined) throw new Error(`${artifactPath} needs a manifest viewport`);
    const expectedSize = CORPUS_VIEWPORT_SIZES[viewport];
    const dimensions = await pngDimensions(screenshotPath);
    if (dimensions.width !== expectedSize.width || dimensions.height !== expectedSize.height) {
      throw new Error(
        `${artifactPath} must be exactly ${expectedSize.width} by ${expectedSize.height} CSS pixels`,
      );
    }
  }
}

async function validateOrCreateDestinationParent(root, artifactPath) {
  const corpusDirectory = path.join(root, ...CORPUS_DIRECTORY.split("/"));
  const corpusMetadata = await lstat(corpusDirectory);
  if (!corpusMetadata.isDirectory() || corpusMetadata.isSymbolicLink()) {
    throw new Error("docs/screenshots must be a regular directory");
  }
  const relativePath = temporaryRelativePath(artifactPath);
  const relativeParent = path.posix.dirname(relativePath);
  let current = corpusDirectory;
  if (relativeParent !== ".") {
    for (const part of relativeParent.split("/")) {
      current = path.join(current, part);
      try {
        await mkdir(current, { mode: 0o755 });
      } catch (error) {
        if (!isPathError(error, "EEXIST")) throw error;
      }
      const metadata = await lstat(current);
      if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
        throw new Error(`corpus parent is unsafe: ${current}`);
      }
    }
  }
  return path.join(root, ...artifactPath.split("/"));
}

async function refuseUnsafeExistingDestination(destination) {
  try {
    const metadata = await lstat(destination);
    if (!metadata.isFile() || metadata.isSymbolicLink()) {
      throw new Error(`corpus destination is unsafe: ${destination}`);
    }
  } catch (error) {
    if (isPathError(error, "ENOENT")) return;
    throw error;
  }
}

async function copyScreenshotsAtomically(root, directory, artifactPaths) {
  for (const artifactPath of artifactPaths) {
    const sourcePath = path.join(directory, ...temporaryRelativePath(artifactPath).split("/"));
    const destinationPath = await validateOrCreateDestinationParent(root, artifactPath);
    await refuseUnsafeExistingDestination(destinationPath);
    const temporaryPath = path.join(
      path.dirname(destinationPath),
      `.${path.basename(destinationPath)}.${process.pid}.${Date.now().toString(36)}.tmp`,
    );
    await copyFile(sourcePath, temporaryPath, fsConstants.COPYFILE_EXCL);
    await chmod(temporaryPath, 0o644);
    await rename(temporaryPath, destinationPath);
  }
}

function validateOwnerPipeline(owner, pipeline, artifactPaths) {
  if (artifactPaths.length === 0) throw new Error(`capture owner has no artifacts: ${owner}`);
  for (const artifactPath of artifactPaths) {
    const surface = surfaceForArtifact(artifactPath);
    if (surface === undefined || surface.captureOwner !== owner || surface.pipeline !== pipeline) {
      throw new Error(`capture owner or pipeline disagrees for ${artifactPath}`);
    }
  }
}

/** Run one command-specific producer through the shared corpus lifecycle. */
export async function runCorpusCapture(options) {
  const artifactPaths = artifactPathsForCaptureOwner(options.owner);
  validateOwnerPipeline(options.owner, options.pipeline, artifactPaths);
  const directory = await mkdtemp(path.join(PRIVATE_TEMPORARY_PARENT, PRIVATE_TEMPORARY_PREFIX));
  await chmod(directory, 0o700);
  try {
    await validatePrivateDirectory(directory, true);
    await options.runCapture(directory);
    await validateCapturedScreenshots(directory, artifactPaths);
    if (options.mode === "refresh") {
      await copyScreenshotsAtomically(options.root, directory, artifactPaths);
      await recordProvenance(options.root, options.pipeline, options.owner, artifactPaths);
    }
  } finally {
    await validatePrivateDirectory(directory, true);
    await rm(directory, { recursive: true, force: false });
  }
  const result = options.mode === "refresh" ? "refreshed" : "verified without changing docs";
  process.stdout.write(`PASS: ${options.label} ${result}.\n`);
}
