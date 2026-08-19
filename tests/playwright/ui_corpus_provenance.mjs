// Provenance and recursive reconciliation for the committed UI screenshot corpus.

import { execFile } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { chmod, lstat, open, readdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

import {
  artifactPathsForCaptureOwner,
  captureOwnerForArtifact,
  CORPUS_VIEWPORT_SIZES,
  CORPUS_DIRECTORY,
  manifestArtifactPaths,
  surfaceForArtifact,
  surfacesWithRequiredViewports,
  validateCorpusArtifactPath,
  viewportForArtifact,
} from "./ui_corpus_manifest.ts";

const execFileAsync = promisify(execFile);

export const CORPUS_OWNING_PATHS = ["src"];
export const CORPUS_PROVENANCE_FILE = path.posix.join(CORPUS_DIRECTORY, "corpus_provenance.json");

async function git(root, args) {
  const { stdout } = await execFileAsync("git", args, { cwd: root });
  return stdout.trim();
}

export async function currentTreeIdentity(root) {
  return await git(root, ["rev-parse", "HEAD"]);
}

function repositoryPath(...parts) {
  return parts.join("/");
}

async function collectPngPaths(root, relativeDirectory) {
  const directory = path.join(root, ...relativeDirectory.split("/"));
  const metadata = await lstat(directory);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new Error(`corpus directory is unsafe: ${relativeDirectory}`);
  }
  const entries = await readdir(directory, { withFileTypes: true });
  const pngPaths = [];
  for (const entry of entries) {
    const relativePath = repositoryPath(relativeDirectory, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error(`corpus contains a symbolic link: ${relativePath}`);
    }
    if (entry.isDirectory()) {
      pngPaths.push(...(await collectPngPaths(root, relativePath)));
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".png")) pngPaths.push(relativePath);
  }
  return pngPaths;
}

/** Repository-relative PNG paths present recursively in the corpus directory. */
export async function committedArtifactPaths(root) {
  return (await collectPngPaths(root, CORPUS_DIRECTORY)).sort();
}

export async function readProvenance(root) {
  const target = path.join(root, ...CORPUS_PROVENANCE_FILE.split("/"));
  const raw = await readFile(target, "utf8").catch(() => undefined);
  if (raw === undefined) return { artifacts: {} };
  const parsed = JSON.parse(raw);
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    typeof parsed.artifacts !== "object" ||
    parsed.artifacts === null ||
    Array.isArray(parsed.artifacts)
  ) {
    throw new Error("corpus provenance must contain one artifacts object");
  }
  return parsed;
}

async function writeProvenanceAtomically(root, record) {
  const corpusDirectory = path.join(root, ...CORPUS_DIRECTORY.split("/"));
  const metadata = await lstat(corpusDirectory);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new Error("docs/screenshots must be a regular directory");
  }
  const target = path.join(root, ...CORPUS_PROVENANCE_FILE.split("/"));
  const temporary = path.join(
    corpusDirectory,
    `.corpus_provenance.${process.pid}.${Date.now().toString(36)}.tmp`,
  );
  await writeFile(temporary, `${JSON.stringify(record, null, 2)}\n`, {
    encoding: "utf8",
    flag: "wx",
    mode: 0o644,
  });
  await chmod(temporary, 0o644);
  await rename(temporary, target);
}

function artifactLocation(root, artifactPath) {
  return path.join(root, ...artifactPath.split("/"));
}

async function fileContentDigest(filePath) {
  const content = await readFile(filePath);
  const digest = createHash("sha256").update(content).digest("hex");
  return digest;
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
    const dimensions = { width: header.readUInt32BE(16), height: header.readUInt32BE(20) };
    return dimensions;
  } finally {
    await handle.close();
  }
}

/** Stamp exactly the artifacts a successful non-verify capture runner copied. */
export async function recordProvenance(root, pipeline, captureOwner, artifactPaths) {
  const existing = await readProvenance(root);
  const treeIdentity = await currentTreeIdentity(root);
  const capturedAt = new Date().toISOString();
  const captureGeneration = randomUUID();
  const artifacts = { ...existing.artifacts };
  for (const artifactPath of artifactPaths) {
    validateCorpusArtifactPath(artifactPath);
    const surface = surfaceForArtifact(artifactPath);
    if (
      surface === undefined ||
      surface.pipeline !== pipeline ||
      surface.captureOwner !== captureOwner
    ) {
      throw new Error(`capture pipeline does not own ${artifactPath}`);
    }
    const contentDigest = await fileContentDigest(artifactLocation(root, artifactPath));
    artifacts[artifactPath] = {
      pipeline,
      captureOwner,
      captureGeneration,
      contentDigest,
      viewport: viewportForArtifact(artifactPath),
      treeIdentity,
      capturedAt,
    };
  }
  const record = { artifacts };
  await writeProvenanceAtomically(root, record);
  return record;
}

/** Seed only missing provenance from committed image history. */
export async function bootstrapProvenanceFromHistory(root) {
  const existing = await readProvenance(root);
  const artifacts = { ...existing.artifacts };
  for (const artifactPath of manifestArtifactPaths()) {
    if (artifacts[artifactPath] !== undefined) continue;
    const line = await git(root, ["log", "-1", "--format=%H %cI", "--", artifactPath]);
    if (line === "") continue;
    const [treeIdentity, capturedAt] = line.split(" ");
    const surface = surfaceForArtifact(artifactPath);
    artifacts[artifactPath] = {
      pipeline: surface?.pipeline,
      viewport: viewportForArtifact(artifactPath),
      treeIdentity,
      capturedAt,
      derivedFromHistory: true,
    };
  }
  const record = { artifacts };
  await writeProvenanceAtomically(root, record);
  return record;
}

export async function reconcileCorpus(root) {
  const declared = [...manifestArtifactPaths()].sort();
  const committed = await committedArtifactPaths(root);
  const declaredSet = new Set(declared);
  const committedSet = new Set(committed);
  const missing = declared.filter((artifactPath) => !committedSet.has(artifactPath));
  const unowned = committed.filter((artifactPath) => !declaredSet.has(artifactPath));
  return { declared, committed, missing, unowned };
}

/** Fail-worthy gaps in explicitly required visual matrices and their capture provenance. */
export async function requiredVisualEvidenceIssues(root) {
  const provenance = await readProvenance(root);
  const committed = new Set(await committedArtifactPaths(root));
  const issues = [];
  const requiredCaptureOwners = new Set();
  for (const surface of surfacesWithRequiredViewports()) {
    requiredCaptureOwners.add(surface.captureOwner);
    for (const artifact of surface.artifacts) {
      if (!committed.has(artifact.path)) {
        issues.push(`${artifact.path}: required artifact is missing`);
        continue;
      }
      const entry = provenance.artifacts[artifact.path];
      if (entry === undefined) {
        issues.push(`${artifact.path}: required provenance is missing`);
        continue;
      }
      if (entry.pipeline !== surface.pipeline || entry.viewport !== artifact.viewport) {
        issues.push(
          `${artifact.path}: provenance pipeline or viewport disagrees with the manifest`,
        );
      }
      const expectedSize = CORPUS_VIEWPORT_SIZES[artifact.viewport];
      try {
        const dimensions = await pngDimensions(artifactLocation(root, artifact.path));
        if (dimensions.width !== expectedSize.width || dimensions.height !== expectedSize.height) {
          issues.push(
            `${artifact.path}: committed PNG dimensions must be exactly ${expectedSize.width} by ${expectedSize.height}`,
          );
        }
      } catch (error) {
        const detail = error instanceof Error ? error.message : "unknown PNG header error";
        issues.push(`${artifact.path}: committed PNG header is invalid: ${detail}`);
      }
      if (
        typeof entry.treeIdentity !== "string" ||
        entry.treeIdentity.length === 0 ||
        typeof entry.capturedAt !== "string" ||
        entry.capturedAt.length === 0
      ) {
        issues.push(`${artifact.path}: provenance capture identity is incomplete`);
      }
    }
  }
  for (const captureOwner of requiredCaptureOwners) {
    issues.push(
      ...(await captureGenerationIntegrityIssues(root, provenance, committed, captureOwner)),
    );
  }
  return issues;
}

/** Verify every capture owner that has a content-bound generation, not only required matrices. */
export async function contentBoundCaptureGenerationIssues(root) {
  const provenance = await readProvenance(root);
  const committed = new Set(await committedArtifactPaths(root));
  const owners = new Set();
  for (const artifactPath of manifestArtifactPaths()) {
    const entry = provenance.artifacts[artifactPath];
    if (typeof entry?.captureGeneration === "string") {
      const captureOwner = captureOwnerForArtifact(artifactPath);
      if (captureOwner !== undefined) owners.add(captureOwner);
    }
  }
  const issues = [];
  for (const captureOwner of owners) {
    issues.push(
      ...(await captureGenerationIntegrityIssues(root, provenance, committed, captureOwner)),
    );
  }
  return issues;
}

async function captureGenerationIntegrityIssues(root, provenance, committed, captureOwner) {
  const artifactPaths = artifactPathsForCaptureOwner(captureOwner);
  const generations = new Set();
  const issues = [];
  for (const artifactPath of artifactPaths) {
    if (!committed.has(artifactPath)) {
      issues.push(`${artifactPath}: ${captureOwner} capture generation is missing an artifact`);
      continue;
    }
    const entry = provenance.artifacts[artifactPath];
    if (
      entry === undefined ||
      entry.captureOwner !== captureOwner ||
      typeof entry.captureGeneration !== "string" ||
      entry.captureGeneration.length === 0 ||
      typeof entry.contentDigest !== "string" ||
      !/^[a-f0-9]{64}$/u.test(entry.contentDigest)
    ) {
      issues.push(`${artifactPath}: ${captureOwner} capture generation provenance is incomplete`);
      continue;
    }
    const declaredOwner = captureOwnerForArtifact(artifactPath);
    if (declaredOwner !== captureOwner) {
      issues.push(`${artifactPath}: manifest capture owner disagrees with provenance`);
      continue;
    }
    const digest = await fileContentDigest(artifactLocation(root, artifactPath));
    if (digest !== entry.contentDigest) {
      issues.push(
        `${artifactPath}: committed bytes disagree with ${captureOwner} capture generation`,
      );
      continue;
    }
    generations.add(entry.captureGeneration);
  }
  if (generations.size > 1) {
    issues.push(`${captureOwner}: capture generation is mixed across its owned artifacts`);
  }
  return issues;
}

export async function staleness(root) {
  const provenance = await readProvenance(root);
  const reports = [];
  for (const artifactPath of manifestArtifactPaths()) {
    const entry = provenance.artifacts[artifactPath];
    if (entry === undefined) {
      reports.push({ path: artifactPath, state: "unrecorded" });
      continue;
    }
    const range = `${entry.treeIdentity}..HEAD`;
    const output = await git(root, [
      "rev-list",
      "--count",
      range,
      "--",
      ...CORPUS_OWNING_PATHS,
    ]).catch(() => undefined);
    if (output === undefined) {
      reports.push({ path: artifactPath, state: "unknownTree", treeIdentity: entry.treeIdentity });
      continue;
    }
    const commitsSince = Number.parseInt(output, 10);
    reports.push({
      path: artifactPath,
      state: commitsSince > 0 ? "sourceMovedSinceCapture" : "current",
      commitsSince,
      capturedAt: entry.capturedAt,
      treeIdentity: entry.treeIdentity,
    });
  }
  return reports;
}

export async function summarize(root) {
  const { declared, committed, missing, unowned } = await reconcileCorpus(root);
  const reports = await staleness(root);
  const lines = [];
  lines.push(`corpus: ${committed.length} committed, ${declared.length} declared by the manifest`);
  for (const artifactPath of missing) {
    const owner = surfaceForArtifact(artifactPath);
    lines.push(`  declared without a committed file: ${artifactPath} (surface ${owner?.surface})`);
  }
  for (const artifactPath of unowned) {
    lines.push(`  committed without a manifest owner: ${artifactPath}`);
  }
  const unrecorded = reports.filter((report) => report.state === "unrecorded").length;
  const moved = reports.filter((report) => report.state === "sourceMovedSinceCapture");
  lines.push(`provenance: ${reports.length - unrecorded} recorded, ${unrecorded} unrecorded`);
  for (const report of moved) {
    lines.push(
      `  source moved since capture: ${report.path} (${report.commitsSince} commits touching ${CORPUS_OWNING_PATHS.join(", ")})`,
    );
  }
  return lines.join("\n");
}
