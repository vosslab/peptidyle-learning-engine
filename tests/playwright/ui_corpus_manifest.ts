// Canonical UI visual evidence loaded from the shared JSON corpus authority.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const CORPUS_DIRECTORY = "docs/screenshots";

export interface CorpusViewport {
  readonly width: number;
  readonly height: number;
  readonly deviceScaleFactor: number;
}

export interface CorpusArtifact {
  readonly artifactId: string;
  readonly scenarioId: string;
  readonly stateId: string;
  readonly path: string;
  readonly viewport: keyof typeof CORPUS_VIEWPORT_SIZES;
  readonly role: string;
  readonly journey: string;
  readonly captureOrder: number;
  readonly journeyStep: number;
  readonly privacyChecks: readonly string[];
}

interface CorpusSource {
  readonly schemaVersion: number;
  readonly corpusDirectory: string;
  readonly viewportProfiles: Record<string, CorpusViewport>;
  readonly artifacts: readonly CorpusArtifact[];
}

function loadCorpusSource(): CorpusSource {
  const sourcePath = fileURLToPath(
    new URL("../e2e/browser_screenshot_corpus.json", import.meta.url),
  );
  const value: unknown = JSON.parse(readFileSync(sourcePath, "ascii"));
  if (!isCorpusSource(value)) throw new Error("real-stack screenshot corpus JSON is invalid");
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isViewport(value: unknown): value is CorpusViewport {
  if (!isRecord(value)) return false;
  return (
    typeof value.width === "number" &&
    typeof value.height === "number" &&
    typeof value.deviceScaleFactor === "number"
  );
}

function isArtifact(value: unknown): value is CorpusArtifact {
  if (!isRecord(value)) return false;
  return (
    typeof value.artifactId === "string" &&
    typeof value.scenarioId === "string" &&
    typeof value.stateId === "string" &&
    typeof value.path === "string" &&
    typeof value.viewport === "string" &&
    typeof value.role === "string" &&
    typeof value.journey === "string" &&
    typeof value.captureOrder === "number" &&
    typeof value.journeyStep === "number" &&
    Array.isArray(value.privacyChecks) &&
    value.privacyChecks.every((item) => typeof item === "string")
  );
}

function isCorpusSource(value: unknown): value is CorpusSource {
  if (!isRecord(value) || !isRecord(value.viewportProfiles) || !Array.isArray(value.artifacts)) {
    return false;
  }
  return (
    typeof value.schemaVersion === "number" &&
    typeof value.corpusDirectory === "string" &&
    Object.values(value.viewportProfiles).every(isViewport) &&
    value.artifacts.every(isArtifact)
  );
}

const corpusSource = loadCorpusSource();
export const CORPUS_VIEWPORT_SIZES = corpusSource.viewportProfiles as Record<
  "laptop" | "tablet" | "iphone_pro" | "square",
  CorpusViewport
>;
export const UI_CORPUS_MANIFEST = corpusSource.artifacts;

export function manifestArtifactPaths(): readonly string[] {
  return UI_CORPUS_MANIFEST.map((artifact) => artifact.path);
}

export function validateManifest(): void {
  const viewportNames = Object.keys(CORPUS_VIEWPORT_SIZES);
  if (
    corpusSource.schemaVersion !== 2 ||
    corpusSource.corpusDirectory !== CORPUS_DIRECTORY ||
    viewportNames.length !== 4 ||
    UI_CORPUS_MANIFEST.length === 0
  ) {
    throw new Error("real-stack screenshot corpus has an invalid top-level contract");
  }
  const ids = new Set<string>();
  const paths = new Set<string>();
  for (const [index, artifact] of UI_CORPUS_MANIFEST.entries()) {
    if (
      ids.has(artifact.artifactId) ||
      paths.has(artifact.path) ||
      artifact.captureOrder !== index + 1 ||
      !(artifact.viewport in CORPUS_VIEWPORT_SIZES) ||
      !artifact.path.startsWith(`${CORPUS_DIRECTORY}/`) ||
      artifact.privacyChecks[0] !== "no_private_material"
    ) {
      throw new Error("real-stack screenshot corpus has an invalid artifact");
    }
    ids.add(artifact.artifactId);
    paths.add(artifact.path);
  }
}

validateManifest();
