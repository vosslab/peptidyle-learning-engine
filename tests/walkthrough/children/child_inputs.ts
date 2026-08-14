// child_inputs.ts - strict explicit private handoff for fixed walkthrough children.

import { lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute } from "node:path";

import {
  isAssignmentReference,
  isCourseReference,
} from "../../playwright/simulator/public_references";

const MAX_INPUT_BYTES = 8192;
export type WalkthroughInputStage = "arrangement" | "learner_journey";

interface CommonWalkthroughInputs {
  readonly schemaVersion: 1;
}

/** The arranger needs only the launcher manifest that names its four questions. */
export interface ArrangementChildInputs extends CommonWalkthroughInputs {
  readonly stage: "arrangement";
  readonly chapterOneManifestFile: string;
}

/** The report and cross-actor children need durable journey state and its seed. */
export interface LearnerJourneyChildInputs extends CommonWalkthroughInputs {
  readonly stage: "learner_journey";
  readonly journeyStateFile: string;
  readonly masterSeed: number;
}

export type WalkthroughChildInputs = ArrangementChildInputs | LearnerJourneyChildInputs;

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function requireString(value: unknown): string {
  if (typeof value !== "string" || value === "") throw new Error("walkthrough-inputs");
  return value;
}

function requirePrivatePath(value: unknown): string {
  const path = requireString(value);
  if (!isAbsolute(path)) throw new Error("walkthrough-inputs");
  return path;
}

function optionalPrivatePath(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  const path = requirePrivatePath(value);
  return path;
}

function requireReference(
  value: unknown,
  validator: (candidate: unknown) => candidate is string,
): string {
  if (!validator(value)) throw new Error("walkthrough-inputs");
  return value;
}

function expectedKeys(stage: WalkthroughInputStage): readonly string[] {
  switch (stage) {
    case "arrangement":
      return ["schemaVersion", "stage", "chapterOneManifestFile"];
    case "learner_journey":
      return [
        "schemaVersion",
        "stage",
        "baseUrl",
        "masterSeed",
        "credentialFile",
        "journeyStateFile",
        "j1CheckpointFile",
        "j2CheckpointFile",
        "courseReference",
        "masteryAssignmentReference",
        "screenshotDirectory",
      ];
  }
}

function validateExactKeys(
  value: Readonly<Record<string, unknown>>,
  stage: WalkthroughInputStage,
): void {
  const expected = expectedKeys(stage);
  if (
    Object.keys(value).length !== expected.length ||
    !expected.every((key) => Object.prototype.hasOwnProperty.call(value, key))
  ) {
    throw new Error("walkthrough-inputs");
  }
}

function parseInputs(value: unknown): WalkthroughChildInputs {
  if (!isRecord(value) || value["schemaVersion"] !== 1) throw new Error("walkthrough-inputs");
  const stage = value["stage"];
  if (stage !== "arrangement" && stage !== "learner_journey") {
    throw new Error("walkthrough-inputs");
  }
  validateExactKeys(value, stage);
  if (stage === "arrangement") {
    return {
      schemaVersion: 1,
      stage,
      chapterOneManifestFile: requirePrivatePath(value["chapterOneManifestFile"]),
    };
  }
  const masterSeedValue = value["masterSeed"];
  if (
    typeof masterSeedValue !== "number" ||
    !Number.isInteger(masterSeedValue) ||
    masterSeedValue < 0 ||
    masterSeedValue > 0xffffffff
  ) {
    throw new Error("walkthrough-inputs");
  }
  const masterSeed = masterSeedValue;
  requireString(value["baseUrl"]);
  requirePrivatePath(value["credentialFile"]);
  optionalPrivatePath(value["screenshotDirectory"]);
  requirePrivatePath(value["j1CheckpointFile"]);
  requirePrivatePath(value["j2CheckpointFile"]);
  requireReference(value["courseReference"], isCourseReference);
  requireReference(value["masteryAssignmentReference"], isAssignmentReference);
  return {
    schemaVersion: 1,
    stage,
    masterSeed,
    journeyStateFile: requirePrivatePath(value["journeyStateFile"]),
  };
}

function inputPathFromArguments(argv: readonly string[]): string {
  if (argv.length !== 2 || argv[0] !== "--inputs") throw new Error("walkthrough-inputs");
  const path = argv[1];
  if (path === undefined) throw new Error("walkthrough-inputs");
  return requirePrivatePath(path);
}

function readPrivateInputsFile(path: string): string {
  const metadata = lstatSync(path);
  const parent = lstatSync(dirname(path));
  if (
    !metadata.isFile() ||
    metadata.isSymbolicLink() ||
    (metadata.mode & 0o777) !== 0o600 ||
    !parent.isDirectory() ||
    parent.isSymbolicLink() ||
    (parent.mode & 0o777) !== 0o700
  ) {
    throw new Error("walkthrough-inputs");
  }
  const bytes = readFileSync(path);
  if (bytes.length === 0 || bytes.length > MAX_INPUT_BYTES || bytes.some((byte) => byte > 0x7f)) {
    throw new Error("walkthrough-inputs");
  }
  return bytes.toString("ascii");
}

export function childInputsFromArguments(
  argv: readonly string[],
  expectedStage: "arrangement",
): ArrangementChildInputs;
export function childInputsFromArguments(
  argv: readonly string[],
  expectedStage: "learner_journey",
): LearnerJourneyChildInputs;
export function childInputsFromArguments(
  argv: readonly string[],
  expectedStage: WalkthroughInputStage,
): WalkthroughChildInputs {
  const path = inputPathFromArguments(argv);
  const text = readPrivateInputsFile(path);
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    throw new Error("walkthrough-inputs");
  }
  if (text !== JSON.stringify(raw)) throw new Error("walkthrough-inputs");
  const inputs = parseInputs(raw);
  if (inputs.stage !== expectedStage) throw new Error("walkthrough-inputs");
  return inputs;
}
