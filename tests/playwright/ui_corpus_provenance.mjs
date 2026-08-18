// Provenance and reconciliation for the committed UI screenshot corpus.
//
// The corpus previously carried no record of the code state each image was captured from, so
// answering "is this current?" required re-running an expensive pipeline. Recording the capture
// commit makes that question answerable by inspection.
//
// Staleness is reported rather than enforced. A spike against the current corpus measured every
// candidate owning path -- src, src/style.css, src/pages, src/components, src/features -- and found
// they share one last-change commit, because this repository lands large batched commits. Narrowing
// the path therefore adds no discrimination, so the useful signal is the plain count of commits
// touching the browser sources since capture.

import { execFile } from "node:child_process";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

import {
  CORPUS_DIRECTORY,
  manifestArtifactNames,
  surfaceForArtifact,
  viewportForArtifact,
} from "./ui_corpus_manifest.ts";

const execFileAsync = promisify(execFile);

/** Browser sources whose changes can invalidate a captured surface. */
export const CORPUS_OWNING_PATHS = ["src"];

/** Repository-relative provenance record committed beside the corpus. */
export const CORPUS_PROVENANCE_FILE = path.join(CORPUS_DIRECTORY, "corpus_provenance.json");

async function git(root, args) {
  const { stdout } = await execFileAsync("git", args, { cwd: root });
  return stdout.trim();
}

/** Current repository tree identity, used to stamp a capture. */
export async function currentTreeIdentity(root) {
  const commit = await git(root, ["rev-parse", "HEAD"]);
  return commit;
}

/** Committed PNG names present in the corpus directory. */
export async function committedArtifactNames(root) {
  const entries = await readdir(path.join(root, CORPUS_DIRECTORY));
  const names = entries.filter((entry) => entry.endsWith(".png")).sort();
  return names;
}

/** Read the provenance record, returning an empty record when none exists yet. */
export async function readProvenance(root) {
  const target = path.join(root, CORPUS_PROVENANCE_FILE);
  const raw = await readFile(target, "utf8").catch(() => undefined);
  if (raw === undefined) return { artifacts: {} };
  const parsed = JSON.parse(raw);
  return parsed;
}

/** Write the provenance record for the artifacts one pipeline just captured. */
export async function recordProvenance(root, pipeline, names) {
  const existing = await readProvenance(root);
  const treeIdentity = await currentTreeIdentity(root);
  const capturedAt = new Date().toISOString();
  const artifacts = { ...existing.artifacts };
  for (const name of names) {
    artifacts[name] = {
      pipeline,
      viewport: viewportForArtifact(name),
      treeIdentity,
      capturedAt,
    };
  }
  const record = { artifacts };
  const target = path.join(root, CORPUS_PROVENANCE_FILE);
  await writeFile(target, `${JSON.stringify(record, null, 2)}\n`, "utf8");
  return record;
}

/**
 * Seed provenance for artifacts captured before this record existed.
 *
 * Each entry takes the last commit that touched its image, which is the closest available stand-in
 * for the code state it was captured from. Entries are marked as derived so a reader can tell them
 * apart from a stamp written by an actual capture run.
 */
export async function bootstrapProvenanceFromHistory(root) {
  const existing = await readProvenance(root);
  const artifacts = { ...existing.artifacts };
  for (const name of manifestArtifactNames()) {
    if (artifacts[name] !== undefined) continue;
    const target = `${CORPUS_DIRECTORY}/${name}`;
    const line = await git(root, ["log", "-1", "--format=%H %cI", "--", target]);
    if (line === "") continue;
    const [treeIdentity, capturedAt] = line.split(" ");
    const surface = surfaceForArtifact(name);
    artifacts[name] = {
      pipeline: surface?.pipeline,
      viewport: viewportForArtifact(name),
      treeIdentity,
      capturedAt,
      derivedFromHistory: true,
    };
  }
  const record = { artifacts };
  await writeFile(
    path.join(root, CORPUS_PROVENANCE_FILE),
    `${JSON.stringify(record, null, 2)}\n`,
    "utf8",
  );
  return record;
}

/**
 * Reconcile the manifest against the committed corpus.
 *
 * Reports artifacts the manifest declares but the corpus lacks, and files the corpus holds that no
 * surface owns. An unowned file is how one image drifted into the corpus with neither pipeline
 * producing it and no document citing it.
 */
export async function reconcileCorpus(root) {
  const declared = manifestArtifactNames();
  const committed = await committedArtifactNames(root);
  const declaredSet = new Set(declared);
  const committedSet = new Set(committed);
  const missing = declared.filter((name) => !committedSet.has(name));
  const unowned = committed.filter((name) => !declaredSet.has(name));
  return { declared, committed, missing, unowned };
}

/**
 * Count commits touching the browser sources since each artifact was captured.
 *
 * A positive count means the interface may have moved since the image was taken. This is reported
 * as information rather than a failure, because a source change does not always alter a surface.
 */
export async function staleness(root) {
  const provenance = await readProvenance(root);
  const reports = [];
  for (const name of manifestArtifactNames()) {
    const entry = provenance.artifacts[name];
    if (entry === undefined) {
      reports.push({ name, state: "unrecorded" });
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
      reports.push({ name, state: "unknownTree", treeIdentity: entry.treeIdentity });
      continue;
    }
    const commitsSince = Number.parseInt(output, 10);
    reports.push({
      name,
      state: commitsSince > 0 ? "sourceMovedSinceCapture" : "current",
      commitsSince,
      capturedAt: entry.capturedAt,
      treeIdentity: entry.treeIdentity,
    });
  }
  return reports;
}

/** Human-readable reconciliation and staleness summary. */
export async function summarize(root) {
  const { declared, committed, missing, unowned } = await reconcileCorpus(root);
  const reports = await staleness(root);
  const lines = [];
  lines.push(`corpus: ${committed.length} committed, ${declared.length} declared by the manifest`);
  for (const name of missing) {
    const owner = surfaceForArtifact(name);
    lines.push(`  declared without a committed file: ${name} (surface ${owner?.surface})`);
  }
  for (const name of unowned) {
    lines.push(`  committed without a manifest owner: ${name}`);
  }
  const unrecorded = reports.filter((report) => report.state === "unrecorded").length;
  const moved = reports.filter((report) => report.state === "sourceMovedSinceCapture");
  lines.push(`provenance: ${reports.length - unrecorded} recorded, ${unrecorded} unrecorded`);
  for (const report of moved) {
    lines.push(
      `  source moved since capture: ${report.name} (${report.commitsSince} commits touching ${CORPUS_OWNING_PATHS.join(", ")})`,
    );
  }
  return lines.join("\n");
}
