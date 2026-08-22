#!/usr/bin/env node
// Offline integrity check for the committed twenty-image real-stack corpus.
import { createHash } from "node:crypto";
import { constants } from "node:fs";
import { lstat, open, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

import {
  CORPUS_DIRECTORY,
  CORPUS_VIEWPORT_SIZES,
  UI_CORPUS_MANIFEST,
} from "./ui_corpus_manifest.ts";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function pngDimensions(content) {
  if (!content.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])))
    throw new Error("invalid PNG");
  let offset = 8;
  let ihdr = 0;
  let idat = 0;
  let width = 0;
  let height = 0;
  while (offset < content.length) {
    if (content.length - offset < 12) throw new Error("invalid PNG framing");
    const length = content.readUInt32BE(offset);
    const end = offset + 12 + length;
    if (end > content.length) throw new Error("invalid PNG framing");
    const kind = content.subarray(offset + 4, offset + 8);
    const data = content.subarray(offset + 8, offset + 8 + length);
    const expectedCrc = content.readUInt32BE(offset + 8 + length);
    const actualCrc = crc32(Buffer.concat([kind, data]));
    if (actualCrc !== expectedCrc) throw new Error("invalid PNG CRC");
    if (kind.equals(Buffer.from("IHDR"))) {
      ihdr += 1;
      if (ihdr !== 1 || offset !== 8 || length !== 13) throw new Error("invalid PNG IHDR");
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
    } else if (kind.equals(Buffer.from("IDAT"))) idat += 1;
    else if (kind.equals(Buffer.from("IEND"))) {
      if (length !== 0 || end !== content.length) throw new Error("invalid PNG IEND");
      if (ihdr !== 1 || idat < 1) throw new Error("invalid PNG structure");
      return { width, height };
    }
    offset = end;
  }
  throw new Error("invalid PNG IEND");
}

function crc32(content) {
  let crc = 0xffffffff;
  for (const byte of content) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function isDigest(value, length) {
  return typeof value === "string" && new RegExp(`^[0-9a-f]{${length}}$`).test(value);
}

function hasExactKeys(value, keys) {
  return (
    value !== null &&
    typeof value === "object" &&
    Object.keys(value).length === keys.length &&
    keys.every((key) => Object.hasOwn(value, key))
  );
}

function sameIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino;
}

async function readHeldSnapshot(corpus, relative) {
  const segments = relative.split("/");
  const corpusSegments = CORPUS_DIRECTORY.split("/");
  if (
    segments.length <= corpusSegments.length ||
    corpusSegments.some((segment, index) => segments[index] !== segment) ||
    segments.some((segment) => segment === "" || segment === "." || segment === "..")
  )
    throw new Error(`unsafe corpus artifact ${relative}`);
  const heldDirectories = [];
  let directoryPath = path.join(root, CORPUS_DIRECTORY);
  let handle;
  try {
    for (const component of segments.slice(corpusSegments.length, -1)) {
      directoryPath = path.join(directoryPath, component);
      const before = await lstat(directoryPath);
      if (!before.isDirectory() || before.isSymbolicLink())
        throw new Error(`unsafe corpus parent ${relative}`);
      const handle = await open(
        directoryPath,
        constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW,
      );
      const opened = await handle.stat();
      if (!sameIdentity(before, opened)) {
        await handle.close();
        throw new Error(`corpus parent swapped while opening ${relative}`);
      }
      heldDirectories.push({ handle, identity: opened, target: directoryPath });
    }
    const target = path.join(directoryPath, segments.at(-1));
    const before = await lstat(target);
    if (!before.isFile() || before.isSymbolicLink() || before.size === 0)
      throw new Error(`unsafe corpus artifact ${relative}`);
    handle = await open(target, constants.O_RDONLY | constants.O_NOFOLLOW);
    const opened = await handle.stat();
    if (!sameIdentity(before, opened))
      throw new Error(`corpus artifact swapped while opening ${relative}`);
    const chunks = [];
    let position = 0;
    while (position < opened.size) {
      const buffer = Buffer.alloc(Math.min(1_048_576, opened.size - position));
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, position);
      if (bytesRead === 0) throw new Error(`corpus artifact changed while reading ${relative}`);
      chunks.push(buffer.subarray(0, bytesRead));
      position += bytesRead;
    }
    const afterOpen = await handle.stat();
    const afterPath = await lstat(target);
    const afterDirectory = await corpus.handle.stat();
    if (
      !sameIdentity(opened, afterOpen) ||
      !sameIdentity(before, afterPath) ||
      afterOpen.size !== opened.size ||
      afterPath.size !== opened.size ||
      !sameIdentity(afterDirectory, corpus.identity)
    )
      throw new Error(`corpus artifact changed while reading ${relative}`);
    for (const directory of heldDirectories) {
      const afterOpenDirectory = await directory.handle.stat();
      const afterPathDirectory = await lstat(directory.target);
      if (
        !sameIdentity(directory.identity, afterOpenDirectory) ||
        !sameIdentity(directory.identity, afterPathDirectory)
      )
        throw new Error(`corpus parent changed while reading ${relative}`);
    }
    return Buffer.concat(chunks);
  } finally {
    if (handle !== undefined) await handle.close();
    for (const directory of heldDirectories.reverse()) await directory.handle.close();
  }
}

async function framedDistFileDigest(relative, target) {
  const before = await lstat(target);
  if (before.isSymbolicLink() || !before.isFile() || before.size > 32_000_000)
    throw new Error(`unsafe production dist artifact ${relative}`);
  const handle = await open(target, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const opened = await handle.stat();
    if (!sameIdentity(before, opened) || opened.size !== before.size)
      throw new Error(`production dist artifact swapped while opening ${relative}`);
    const digest = createHash("sha256");
    let position = 0;
    while (position < opened.size) {
      const buffer = Buffer.alloc(Math.min(1_048_576, opened.size - position));
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, position);
      if (bytesRead === 0)
        throw new Error(`production dist artifact changed while reading ${relative}`);
      digest.update(buffer.subarray(0, bytesRead));
      position += bytesRead;
    }
    const afterOpen = await handle.stat();
    const afterPath = await lstat(target);
    if (
      !sameIdentity(opened, afterOpen) ||
      !sameIdentity(before, afterPath) ||
      afterOpen.size !== opened.size ||
      afterPath.size !== opened.size
    )
      throw new Error(`production dist artifact changed while reading ${relative}`);
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(opened.size));
    return Buffer.concat([Buffer.from(relative), Buffer.from([0, 70]), length, digest.digest()]);
  } finally {
    await handle.close();
  }
}

async function productionDistDigest() {
  const base = path.join(root, "dist");
  const entries = await readdir(base, { recursive: true, withFileTypes: true });
  const paths = [];
  for (const entry of entries) {
    const relative = path.join(entry.parentPath.slice(base.length + 1), entry.name);
    if (entry.isSymbolicLink() || !entry.isFile()) {
      if (!entry.isDirectory()) throw new Error(`unsafe production dist artifact ${relative}`);
      continue;
    }
    paths.push(relative);
  }
  if (paths.length === 0) throw new Error("production dist is empty");
  const digest = createHash("sha256");
  for (const relative of paths.sort())
    digest.update(await framedDistFileDigest(relative, path.join(base, relative)));
  return digest.digest("hex");
}

async function main() {
  const corpusPath = path.join(root, CORPUS_DIRECTORY);
  const corpusHandle = await open(
    corpusPath,
    constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW,
  );
  const corpus = { handle: corpusHandle, identity: await corpusHandle.stat() };
  try {
    const corpusPathMetadata = await lstat(corpusPath);
    if (
      !corpusPathMetadata.isDirectory() ||
      corpusPathMetadata.isSymbolicLink() ||
      !sameIdentity(corpus.identity, corpusPathMetadata)
    )
      throw new Error("unsafe screenshot corpus directory");
    const provenance = JSON.parse(
      (
        await readHeldSnapshot(corpus, path.join(CORPUS_DIRECTORY, "corpus_provenance.json"))
      ).toString("utf8"),
    );
    if (
      !hasExactKeys(provenance, [
        "schemaVersion",
        "pipeline",
        "browserSuite",
        "origin",
        "productionDistDigest",
        "generationIdentity",
        "artifacts",
      ]) ||
      provenance.schemaVersion !== 2 ||
      provenance.pipeline !== "realStack" ||
      provenance.browserSuite !== "ple-live-demo-browser" ||
      !/^https:\/\/localhost:[1-9][0-9]*$/.test(provenance.origin) ||
      !isDigest(provenance.productionDistDigest, 64) ||
      !isDigest(provenance.generationIdentity, 64) ||
      !Array.isArray(provenance.artifacts) ||
      provenance.artifacts.length !== UI_CORPUS_MANIFEST.length
    )
      throw new Error("corpus provenance is not one complete real-stack generation");
    const records = [];
    for (const [index, artifact] of UI_CORPUS_MANIFEST.entries()) {
      const record = provenance.artifacts[index];
      const viewport = CORPUS_VIEWPORT_SIZES[artifact.viewport];
      if (
        record === undefined ||
        !hasExactKeys(record, [
          "artifactId",
          "scenarioId",
          "stateId",
          "role",
          "journey",
          "captureOrder",
          "journeyStep",
          "viewport",
          "path",
          "sha256",
          "width",
          "height",
          "privacyChecks",
        ]) ||
        record.artifactId !== artifact.artifactId ||
        record.path !== artifact.path ||
        record.scenarioId !== artifact.scenarioId ||
        record.stateId !== artifact.stateId ||
        record.role !== artifact.role ||
        record.journey !== artifact.journey ||
        record.captureOrder !== artifact.captureOrder ||
        record.journeyStep !== artifact.journeyStep ||
        record.viewport?.name !== artifact.viewport ||
        record.viewport?.width !== viewport.width ||
        record.viewport?.height !== viewport.height ||
        record.viewport?.deviceScaleFactor !== viewport.deviceScaleFactor ||
        JSON.stringify(record.privacyChecks) !== JSON.stringify(artifact.privacyChecks)
      )
        throw new Error(`provenance is incomplete for ${artifact.artifactId}`);
      if (
        !isDigest(record.sha256, 64) ||
        record.width !== viewport.width ||
        record.height !== viewport.height
      )
        throw new Error(`provenance has invalid artifact metadata for ${artifact.artifactId}`);
      records.push({
        artifactId: record.artifactId,
        captureOrder: record.captureOrder,
        path: record.path,
        sha256: record.sha256,
        viewport: record.viewport,
      });
      const content = await readHeldSnapshot(corpus, artifact.path);
      const dimensions = pngDimensions(content);
      if (
        dimensions.width !== viewport.width ||
        dimensions.height !== viewport.height ||
        createHash("sha256").update(content).digest("hex") !== record.sha256
      )
        throw new Error(`invalid committed artifact ${artifact.path}`);
    }
    const generation = JSON.stringify({
      artifacts: records,
      origin: provenance.origin,
      productionDistDigest: provenance.productionDistDigest,
    });
    if (createHash("sha256").update(generation).digest("hex") !== provenance.generationIdentity)
      throw new Error("corpus provenance generation identity is invalid");
    if ((await productionDistDigest()) !== provenance.productionDistDigest)
      throw new Error("corpus provenance dist digest is stale");
    const finalDirectory = await corpusHandle.stat();
    const finalPath = await lstat(corpusPath);
    if (!sameIdentity(corpus.identity, finalDirectory) || !sameIdentity(corpus.identity, finalPath))
      throw new Error("screenshot corpus directory changed during verification");
    process.stdout.write("PASS: the declared real-stack browser corpus is complete.\n");
  } finally {
    await corpusHandle.close();
  }
}

await main();
