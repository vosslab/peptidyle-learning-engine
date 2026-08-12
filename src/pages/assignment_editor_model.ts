// assignment_editor_model.ts - local state helpers for revisioned assignment editing.

import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { Capability } from "../../generated/api/Capability";
import type { ProblemPublicId } from "../../generated/api/ProblemPublicId";
import type { ProblemVersionNumber } from "../../generated/api/ProblemVersionNumber";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { AssignmentCapabilityViolation, AssignmentEditorInput } from "../api/contracts";
import { DEFAULT_MASTERY_TIME_LIMIT_SECONDS } from "../../generated/api/DEFAULT_MASTERY_TIME_LIMIT_SECONDS";
import { MAX_ASSIGNMENT_TIME_LIMIT_SECONDS } from "../../generated/api/MAX_ASSIGNMENT_TIME_LIMIT_SECONDS";

export interface AssignmentCatalogRow {
  readonly reference: ProblemVersionRef;
  readonly publicId: ProblemPublicId;
  readonly versionNumber: ProblemVersionNumber;
  readonly title: string;
  readonly backend: QuestionBackend;
}

export interface AssignmentEditorDraft extends AssignmentEditorInput {
  /** Empty until the server creates the immutable assignment record. */
  readonly id: string;
  readonly courseId: string;
  /** Empty in create mode because a new assignment has no server revision yet. */
  readonly revision: string;
}

const MAX_DIRECT_IMPORT_REFERENCES = 50;
/** Must match question_model::catalog::MAX_CATALOG_DISPLAY_NUMBER. */
const MAX_CATALOG_DISPLAY_NUMBER = 2_147_483_647n;
const EXACT_PROBLEM_DISPLAY_REFERENCE = /^P-([1-9][0-9]{0,19})-v([1-9][0-9]{0,19})$/u;

/**
 * Parses the instructor's copy/paste format without accepting an unstable "latest" alias.
 * Commas and line breaks allow a small curated question set to be added in one task.
 */
export function parseExactProblemDisplayReferences(value: string): ReadonlyArray<string> {
  const references = value
    .split(/[\n,]/u)
    .map((reference) => reference.trim())
    .filter((reference) => reference.length > 0);
  if (references.length === 0) {
    throw new Error("Paste at least one question ID, such as P-12-v3.");
  }
  if (references.length > MAX_DIRECT_IMPORT_REFERENCES) {
    throw new Error(`Add at most ${MAX_DIRECT_IMPORT_REFERENCES} question IDs at a time.`);
  }
  const seen = new Set<string>();
  for (const reference of references) {
    const match = EXACT_PROBLEM_DISPLAY_REFERENCE.exec(reference);
    if (
      match === null ||
      BigInt(match[1] ?? "0") > MAX_CATALOG_DISPLAY_NUMBER ||
      BigInt(match[2] ?? "0") > MAX_CATALOG_DISPLAY_NUMBER
    ) {
      throw new Error(`${reference} is not an exact question ID. Use the form P-12-v3.`);
    }
    if (seen.has(reference)) {
      throw new Error(`${reference} appears more than once. Remove the duplicate and try again.`);
    }
    seen.add(reference);
  }
  return references;
}

/**
 * The Fall-pilot mastery policy: complete only when every answer is correct,
 * retain the highest score, and allow fresh unlimited practice runs.
 */
export function createMasteryAssignmentDraft(courseId: string): AssignmentEditorDraft {
  return {
    id: "",
    courseId,
    title: "",
    problems: [],
    policies: {
      completion: { kind: "allCorrect" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
    assignmentTiming: { timeLimitSeconds: DEFAULT_MASTERY_TIME_LIMIT_SECONDS },
    revision: "",
  };
}

export function sameReference(left: ProblemVersionRef, right: ProblemVersionRef): boolean {
  return left.problem === right.problem && left.version === right.version;
}

export function assignmentInput(draft: AssignmentEditorDraft): AssignmentEditorInput {
  return {
    title: draft.title,
    problems: [...draft.problems],
    policies: draft.policies,
    assignmentTiming: draft.assignmentTiming,
  };
}

export interface TimeLimitValidation {
  readonly seconds: number | null;
  readonly error: string | null;
}

/** Preserves a typed minutes value until save while accepting any exact whole-second duration. */
export function minutesToRunTimeLimit(minutesText: string, timed: boolean): TimeLimitValidation {
  if (!timed) return { seconds: null, error: null };
  const normalized = minutesText.trim();
  const match = /^(?<whole>[0-9]+)(?:\.(?<fraction>[0-9]+))?$/u.exec(normalized);
  if (match === null || normalized.length > 100) {
    return { seconds: null, error: "Enter a positive number of minutes, such as 15." };
  }
  const whole = match.groups?.whole ?? "";
  const fraction = match.groups?.fraction ?? "";
  const numerator = BigInt(`${whole}${fraction}`);
  const denominator = 10n ** BigInt(fraction.length);
  const secondsNumerator = numerator * 60n;
  if (numerator === 0n) {
    return { seconds: null, error: "Enter a positive number of minutes, such as 15." };
  }
  if (secondsNumerator % denominator !== 0n) {
    return { seconds: null, error: "Enter minutes that convert to a whole number of seconds." };
  }
  const seconds = secondsNumerator / denominator;
  if (seconds > BigInt(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)) {
    return {
      seconds: null,
      error: `Enter a duration no longer than ${MAX_ASSIGNMENT_TIME_LIMIT_SECONDS} seconds.`,
    };
  }
  return { seconds: Number(seconds), error: null };
}

export function runTimeLimitMinutes(seconds: number | null): string {
  if (seconds === null) return String(DEFAULT_MASTERY_TIME_LIMIT_SECONDS / 60);
  return String(seconds / 60);
}

export function addCatalogReference(
  draft: AssignmentEditorDraft,
  row: AssignmentCatalogRow,
): AssignmentEditorDraft {
  if (draft.problems.some((reference) => sameReference(reference, row.reference))) {
    return draft;
  }
  return { ...draft, problems: [...draft.problems, row.reference] };
}

export function removeCatalogReference(
  draft: AssignmentEditorDraft,
  reference: ProblemVersionRef,
): AssignmentEditorDraft {
  return {
    ...draft,
    problems: draft.problems.filter((candidate) => !sameReference(candidate, reference)),
  };
}

export function moveCatalogReference(
  draft: AssignmentEditorDraft,
  reference: ProblemVersionRef,
  direction: -1 | 1,
): AssignmentEditorDraft {
  const index = draft.problems.findIndex((candidate) => sameReference(candidate, reference));
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= draft.problems.length) return draft;
  const problems = [...draft.problems];
  const current = problems[index];
  const adjacent = problems[nextIndex];
  if (current === undefined || adjacent === undefined) return draft;
  problems[index] = adjacent;
  problems[nextIndex] = current;
  return { ...draft, problems };
}

export function violationMatchesReference(
  violation: AssignmentCapabilityViolation,
  reference: ProblemVersionRef,
): boolean {
  return sameReference(violation.reference, reference);
}

export function capabilityLabel(capability: Capability): string {
  const labels: Readonly<Record<Capability, string>> = {
    algorithmicGeneration: "algorithmic generation",
    clientRendering: "browser rendering",
    serverGrading: "server grading",
    partialCredit: "partial credit",
    hints: "hints",
    perQuestionTiming: "per-question timing",
    printExport: "print export",
    offlinePreview: "offline preview",
  };
  return labels[capability];
}

/** Copyable instructor-facing identity; UUID tuples remain transport-only. */
export function assignmentProblemLabel(row: AssignmentCatalogRow): string {
  return `P-${row.publicId}-v${row.versionNumber}`;
}

export function questionBackendLabel(backend: QuestionBackend): string {
  const labels: Readonly<Record<QuestionBackend, string>> = {
    native: "PLE native",
    webwork: "WeBWorK",
    qti: "QTI",
    h5p: "H5P",
    imathas: "iMathAS",
  };
  return labels[backend];
}
