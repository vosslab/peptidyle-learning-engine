// publication.ts - receipt, gallery, PNG validation, and recoverable corpus promotion.

import { createHash } from "node:crypto";
import type { Dirent } from "node:fs";
import path from "node:path";
import { cp, mkdir, readFile, readdir, rename, rm, stat, writeFile } from "node:fs/promises";

import {
  CANONICAL_VIEWPORTS,
  ROLE_IDS,
  type CaptureManifest,
  type CaptureRecord,
  type ScreenshotRole,
} from "./manifest";

const PNG_SIGNATURE = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ACTIVE_CORPUS_METADATA = [
  "current_capture_manifest.json",
  "current_capture_receipt.json",
] as const;

export interface PublishedImage {
  readonly id: string;
  readonly path: string;
  readonly width: number;
  readonly height: number;
  readonly sha256: string;
}

export interface CaptureReceipt {
  readonly schemaVersion: 1;
  readonly manifestSha256: string;
  readonly paths: ReadonlyArray<string>;
  readonly images: ReadonlyArray<PublishedImage>;
}

function sha256(bytes: Uint8Array | string): string {
  // ASVS 11.4.1 and 11.4.3: SHA-256 binds the manifest and image bytes.
  return createHash("sha256").update(bytes).digest("hex");
}

export async function manifestDigest(manifestPath: string): Promise<string> {
  return sha256(await readFile(manifestPath));
}

export function pngDimensions(bytes: Uint8Array): {
  readonly width: number;
  readonly height: number;
} {
  const buffer = Buffer.from(bytes);
  if (buffer.length < 24 || !buffer.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error("artifact is not a complete PNG header");
  }
  const chunkType = buffer.subarray(12, 16).toString("ascii");
  if (chunkType !== "IHDR") throw new Error("PNG does not begin with an IHDR chunk");
  const width = buffer.readUInt32BE(16);
  const height = buffer.readUInt32BE(20);
  if (width === 0 || height === 0) throw new Error("PNG dimensions must be nonzero");
  return { width, height };
}

function rolePath(root: string, role: ScreenshotRole): string {
  const resolvedRoot = path.resolve(root);
  const target = path.resolve(resolvedRoot, role);
  // ASVS 5.3.2: closed role names and containment prevent traversal.
  if (path.dirname(target) !== resolvedRoot) throw new Error("role path escaped its corpus root");
  return target;
}

async function actualPngPaths(
  root: string,
  allowedMetadata: ReadonlyArray<string>,
): Promise<ReadonlyArray<string>> {
  const rootEntries = await readdir(root, { withFileTypes: true });
  const directories = rootEntries.filter((entry) => entry.isDirectory()).map((entry) => entry.name);
  const files = rootEntries.filter((entry) => entry.isFile()).map((entry) => entry.name);
  const special = rootEntries.filter((entry) => !entry.isDirectory() && !entry.isFile());
  // ASVS 5.3.2: accept only the internally named role folders and publication metadata.
  comparePathSet([...ROLE_IDS].sort(), directories.sort(), "screenshot corpus role folder");
  comparePathSet([...allowedMetadata].sort(), files.sort(), "screenshot corpus root file");
  if (special.length > 0) {
    throw new Error(
      `screenshot corpus root contains unsupported entries: ${special
        .map((entry) => entry.name)
        .sort()
        .join(",")}`,
    );
  }

  const paths: string[] = [];
  for (const role of ROLE_IDS) {
    const directory = rolePath(root, role);
    const entries: Dirent[] = await readdir(directory, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isDirectory()) {
        throw new Error(`active screenshot role folder must stay flat: ${role}/${entry.name}`);
      }
      if (!entry.isFile() || !entry.name.endsWith(".png")) {
        throw new Error(`unmanaged screenshot role entry: ${role}/${entry.name}`);
      }
      paths.push(`${role}/${entry.name}`);
    }
  }
  return paths.sort();
}

function comparePathSet(
  expected: ReadonlyArray<string>,
  actual: ReadonlyArray<string>,
  label: string,
): void {
  const expectedSet = new Set(expected);
  const actualSet = new Set(actual);
  const missing = expected.filter((item) => !actualSet.has(item));
  const unmanaged = actual.filter((item) => !expectedSet.has(item));
  if (missing.length > 0 || unmanaged.length > 0) {
    throw new Error(
      `${label} path set differs; missing=${missing.join(",") || "none"}; ` +
        `unmanaged=${unmanaged.join(",") || "none"}`,
    );
  }
}

async function imageRecord(root: string, capture: CaptureRecord): Promise<PublishedImage> {
  const target = path.resolve(root, capture.path);
  const resolvedRoot = path.resolve(root);
  if (!target.startsWith(`${resolvedRoot}${path.sep}`))
    throw new Error("capture path escaped root");
  const [metadata, bytes] = await Promise.all([stat(target), readFile(target)]);
  if (!metadata.isFile()) throw new Error(`${capture.path} is not a regular file`);
  const dimensions = pngDimensions(bytes);
  const viewport = CANONICAL_VIEWPORTS[capture.viewport];
  if (dimensions.width !== viewport.width || dimensions.height !== viewport.height) {
    throw new Error(
      `${capture.path} is ${String(dimensions.width)}x${String(dimensions.height)}; ` +
        `expected ${String(viewport.width)}x${String(viewport.height)}`,
    );
  }
  return {
    id: capture.id,
    path: capture.path,
    width: dimensions.width,
    height: dimensions.height,
    sha256: sha256(bytes),
  };
}

export async function inspectCorpus(
  root: string,
  manifest: CaptureManifest,
  allowedMetadata: ReadonlyArray<string> = [],
): Promise<ReadonlyArray<PublishedImage>> {
  const expected = manifest.captures.map((capture) => capture.path).sort();
  comparePathSet(expected, await actualPngPaths(root, allowedMetadata), "screenshot corpus");
  const images = await Promise.all(manifest.captures.map((capture) => imageRecord(root, capture)));
  const pathsByHash = new Map<string, string>();
  for (const image of images) {
    const duplicateOf = pathsByHash.get(image.sha256);
    if (duplicateOf !== undefined) {
      throw new Error(`duplicate screenshot bytes: ${duplicateOf} and ${image.path}`);
    }
    pathsByHash.set(image.sha256, image.path);
  }
  return images.sort((left, right) => left.path.localeCompare(right.path));
}

export function createReceipt(
  digest: string,
  images: ReadonlyArray<PublishedImage>,
): CaptureReceipt {
  const ordered = [...images].sort((left, right) => left.path.localeCompare(right.path));
  return {
    schemaVersion: 1,
    manifestSha256: digest,
    paths: ordered.map((image) => image.path),
    images: ordered,
  };
}

export function receiptJson(receipt: CaptureReceipt): string {
  return `${JSON.stringify(receipt, null, 2)}\n`;
}

function titleCase(value: string): string {
  return value.replace(
    /(^|\s)([a-z])/gu,
    (_match, prefix: string, letter: string) => `${prefix}${letter.toUpperCase()}`,
  );
}

function imageTile(capture: CaptureRecord, imagePrefix: string): string {
  const source = `${imagePrefix}${capture.path}`;
  const featured = capture.gallery.featured ? "<br>Featured" : "";
  return `[![${capture.gallery.caption}](${source})](${source})<br>${capture.gallery.caption}<br>${capture.state} - ${capture.viewport}${featured}`;
}

function pushImageGrid(
  lines: string[],
  captures: ReadonlyArray<CaptureRecord>,
  imagePrefix: string,
): void {
  lines.push("| | | |", "| --- | --- | --- |");
  for (let index = 0; index < captures.length; index += 3) {
    const cells = captures
      .slice(index, index + 3)
      .map((capture) => imageTile(capture, imagePrefix));
    while (cells.length < 3) cells.push("");
    lines.push(`| ${cells.join(" | ")} |`);
  }
  lines.push("");
}

function coverageDetail(entry: CaptureManifest["coverage"]["routes"][number]): string {
  if (entry.status === "captured") return entry.captureIds.join(", ");
  if (entry.status === "covered_by") return `${entry.target}: ${entry.reason}`;
  return entry.reason;
}

function pushCoverageTable(
  lines: string[],
  title: string,
  entries: CaptureManifest["coverage"]["routes"],
): void {
  lines.push(`## ${title}`, "", "| Surface | Status | Evidence or reason |", "| --- | --- | --- |");
  for (const entry of entries) {
    lines.push(`| ${entry.id} | ${entry.status} | ${coverageDetail(entry)} |`);
  }
  lines.push("");
}

export function renderAtlas(manifest: CaptureManifest, imagePrefix: string): string {
  const lines = [
    "# Screenshot atlas",
    "",
    "<!-- Generated from docs/screenshots/current_capture_manifest.json. Do not edit by hand. -->",
    "",
    "This atlas surveys the current reproducible Live Demo surfaces. Machine verification proves",
    "the declared semantic states and privacy boundaries; visual launch readiness still requires",
    "human review.",
    "",
  ];
  const ordered = [...manifest.captures].sort(
    (left, right) => left.gallery.order - right.gallery.order,
  );
  for (const role of ROLE_IDS) {
    const roleCaptures = ordered.filter((capture) => capture.role === role);
    lines.push(`## ${titleCase(role)}`, "");
    const areas = [...new Set(roleCaptures.map((capture) => capture.area))];
    for (const area of areas) {
      lines.push(`### ${titleCase(area)}`, "");
      const areaCaptures = roleCaptures.filter((capture) => capture.area === area);
      const workflows = [...new Set(areaCaptures.map((capture) => capture.workflow))];
      for (const workflow of workflows) {
        lines.push(`#### ${titleCase(workflow)}`, "");
        const workflowCaptures = areaCaptures.filter((capture) => capture.workflow === workflow);
        pushImageGrid(lines, workflowCaptures, imagePrefix);
      }
    }
  }
  pushCoverageTable(lines, "Route coverage", manifest.coverage.routes);
  pushCoverageTable(lines, "Ribbon destination coverage", manifest.coverage.ribbonDestinations);
  return lines.join("\n");
}

function decodeReceipt(value: unknown): CaptureReceipt {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("capture receipt must be an object");
  }
  const record = value as Record<string, unknown>;
  if (
    record["schemaVersion"] !== 1 ||
    typeof record["manifestSha256"] !== "string" ||
    !/^[0-9a-f]{64}$/u.test(record["manifestSha256"]) ||
    !Array.isArray(record["paths"]) ||
    !Array.isArray(record["images"])
  ) {
    throw new Error("capture receipt has an unsupported shape");
  }
  const images = record["images"].map((value, index): PublishedImage => {
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
      throw new Error(`capture receipt image ${String(index)} must be an object`);
    }
    const image = value as Record<string, unknown>;
    if (
      typeof image["id"] !== "string" ||
      typeof image["path"] !== "string" ||
      typeof image["width"] !== "number" ||
      typeof image["height"] !== "number" ||
      typeof image["sha256"] !== "string" ||
      !/^[0-9a-f]{64}$/u.test(image["sha256"])
    ) {
      throw new Error(`capture receipt image ${String(index)} has invalid fields`);
    }
    return {
      id: image["id"],
      path: image["path"],
      width: image["width"],
      height: image["height"],
      sha256: image["sha256"],
    };
  });
  const paths = record["paths"].map((value) => {
    if (typeof value !== "string") throw new Error("capture receipt path must be text");
    return value;
  });
  return {
    schemaVersion: 1,
    manifestSha256: record["manifestSha256"],
    paths,
    images,
  };
}

export async function verifyPublishedArtifacts(options: {
  readonly screenshotRoot: string;
  readonly manifest: CaptureManifest;
  readonly manifestDigest: string;
  readonly receiptPath: string;
  readonly atlasPath: string;
}): Promise<ReadonlyArray<PublishedImage>> {
  const images = await inspectCorpus(
    options.screenshotRoot,
    options.manifest,
    ACTIVE_CORPUS_METADATA,
  );
  const receipt = decodeReceipt(JSON.parse(await readFile(options.receiptPath, "utf8")) as unknown);
  const expectedReceipt = createReceipt(options.manifestDigest, images);
  if (receiptJson(receipt) !== receiptJson(expectedReceipt)) {
    throw new Error("current_capture_receipt.json does not bind the current manifest and PNGs");
  }
  const expectedAtlas = renderAtlas(options.manifest, "screenshots/");
  if ((await readFile(options.atlasPath, "utf8")) !== expectedAtlas) {
    throw new Error("docs/SCREENSHOT_ATLAS.md is not the deterministic manifest gallery");
  }
  return images;
}

export async function writeReplayArtifacts(options: {
  readonly outputRoot: string;
  readonly manifest: CaptureManifest;
  readonly digest: string;
}): Promise<ReadonlyArray<PublishedImage>> {
  const images = await inspectCorpus(options.outputRoot, options.manifest);
  const receipt = createReceipt(options.digest, images);
  await Promise.all([
    writeFile(
      path.join(options.outputRoot, "current_capture_receipt.json"),
      receiptJson(receipt),
      "utf8",
    ),
    writeFile(
      path.join(options.outputRoot, "SCREENSHOT_ATLAS.md"),
      renderAtlas(options.manifest, ""),
      "utf8",
    ),
  ]);
  return images;
}

async function exists(target: string): Promise<boolean> {
  try {
    await stat(target);
    return true;
  } catch (error: unknown) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return false;
    throw error;
  }
}

async function copyIfPresent(source: string, target: string): Promise<void> {
  if (await exists(source)) await cp(source, target, { recursive: true });
}

/**
 * Promote only a fully inspected staging corpus. Portable filesystems cannot atomically replace
 * four folders plus the atlas, so a complete backup supports rollback and crash recovery.
 * ASVS 2.3.3: an ordinary replacement failure restores the previous complete publication.
 */
export async function promoteCorpus(
  options: {
    readonly stagingRoot: string;
    readonly screenshotRoot: string;
    readonly atlasPath: string;
    readonly manifest: CaptureManifest;
    readonly digest: string;
  },
  movePath: (source: string, target: string) => Promise<void> = rename,
): Promise<void> {
  const images = await writeReplayArtifacts({
    outputRoot: options.stagingRoot,
    manifest: options.manifest,
    digest: options.digest,
  });
  const publicationRoot = path.dirname(options.stagingRoot);
  const backupRoot = path.join(publicationRoot, "publication-backup");
  const buildingBackupRoot = `${backupRoot}.building`;
  if (await exists(backupRoot)) {
    throw new Error(
      `an interrupted screenshot publication needs recovery from ${backupRoot} before retrying`,
    );
  }
  await rm(buildingBackupRoot, { force: true, recursive: true });
  await mkdir(buildingBackupRoot, { recursive: true });
  for (const role of ROLE_IDS) {
    await copyIfPresent(rolePath(options.screenshotRoot, role), rolePath(buildingBackupRoot, role));
  }
  const receiptSource = path.join(options.stagingRoot, "current_capture_receipt.json");
  const receiptTarget = path.join(options.screenshotRoot, "current_capture_receipt.json");
  await copyIfPresent(receiptTarget, path.join(buildingBackupRoot, "current_capture_receipt.json"));
  await copyIfPresent(options.atlasPath, path.join(buildingBackupRoot, "SCREENSHOT_ATLAS.md"));
  await movePath(buildingBackupRoot, backupRoot);
  const atlasTemporary = `${options.atlasPath}.new`;
  await rm(atlasTemporary, { force: true });
  await writeFile(atlasTemporary, renderAtlas(options.manifest, "screenshots/"), "utf8");
  try {
    for (const role of ROLE_IDS) {
      const source = rolePath(options.stagingRoot, role);
      const target = rolePath(options.screenshotRoot, role);
      await rm(target, { force: true, recursive: true });
      await movePath(source, target);
    }
    await movePath(receiptSource, receiptTarget);
    await movePath(atlasTemporary, options.atlasPath);
    const promoted = await inspectCorpus(
      options.screenshotRoot,
      options.manifest,
      ACTIVE_CORPUS_METADATA,
    );
    if (
      receiptJson(createReceipt(options.digest, promoted)) !==
      receiptJson(createReceipt(options.digest, images))
    ) {
      throw new Error("promoted screenshot corpus differs from validated staging bytes");
    }
  } catch (error: unknown) {
    for (const role of ROLE_IDS) {
      const target = rolePath(options.screenshotRoot, role);
      await rm(target, { force: true, recursive: true });
      await copyIfPresent(rolePath(backupRoot, role), target);
    }
    await rm(receiptTarget, { force: true });
    await copyIfPresent(path.join(backupRoot, "current_capture_receipt.json"), receiptTarget);
    await rm(options.atlasPath, { force: true });
    await copyIfPresent(path.join(backupRoot, "SCREENSHOT_ATLAS.md"), options.atlasPath);
    throw error;
  }
  await rm(backupRoot, { force: true, recursive: true });
  await rm(options.stagingRoot, { force: true, recursive: true });
}

export async function prepareOutputRoot(root: string): Promise<void> {
  await rm(root, { force: true, recursive: true });
  await mkdir(root, { recursive: true });
  await Promise.all(ROLE_IDS.map((role) => mkdir(rolePath(root, role), { recursive: true })));
}
