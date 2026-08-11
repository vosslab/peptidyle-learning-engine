// ui_walkthrough_report.ts - fixed child that validates and renders J1 public outcome evidence.

import { readJourneyStatePrefix } from "../playwright/simulator/journey_state";
import {
  parseJourneyFragments,
  renderVisibleOutcomeReport,
  type ArrangementRecord,
} from "../playwright/simulator/visible_outcome_report";

const ARRANGEMENT_KEYS: Readonly<Record<string, readonly string[]>> = {
  "launcher-seeded-enrollment": [],
  "launcher-baseline-assignment": ["baselineAssignmentId"],
  "api-retry-corpus-publication": ["problemId", "versionId"],
  "api-mastery-assignment": ["courseId", "masteryAssignmentId"],
  "api-exam-assignment": ["courseId", "examAssignmentId"],
};

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined || value === "") throw new Error("missing fixed report input");
  return value;
}

function parseArrangements(value: string): readonly ArrangementRecord[] | undefined {
  try {
    const parsed: unknown = JSON.parse(value);
    if (!Array.isArray(parsed)) return undefined;
    const output: ArrangementRecord[] = [];
    for (const item of parsed) {
      if (!isRecord(item) || typeof item.label !== "string" || !isRecord(item.publicIds)) {
        return undefined;
      }
      const expectedKeys = ARRANGEMENT_KEYS[item.label];
      if (expectedKeys === undefined || !sameKeys(Object.keys(item.publicIds), expectedKeys)) {
        return undefined;
      }
      const publicIds: Record<string, string> = {};
      for (const [key, identifier] of Object.entries(item.publicIds)) {
        if (typeof identifier !== "string") return undefined;
        publicIds[key] = identifier;
      }
      output.push({ label: item.label, publicIds });
    }
    return output;
  } catch {
    return undefined;
  }
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function sameKeys(actual: readonly string[], expected: readonly string[]): boolean {
  return actual.length === expected.length && actual.every((key) => expected.includes(key));
}

function main(): void {
  const stateFile = required("PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE");
  const prefix = readJourneyStatePrefix(stateFile);
  const fragments = parseJourneyFragments(prefix);
  const arrangements = parseArrangements(required("PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON"));
  const masterSeed = Number(required("PLE_UI_WALKTHROUGH_MASTER_SEED"));
  if (fragments === undefined || arrangements === undefined)
    throw new Error("invalid fixed report input");
  const rendered = renderVisibleOutcomeReport(masterSeed, arrangements, fragments);
  if (rendered === undefined) throw new Error("invalid fixed report input");
  process.stdout.write(rendered);
}

try {
  main();
} catch {
  process.exitCode = 1;
}
