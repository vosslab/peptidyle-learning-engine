// assignment_editor_model.ts - browser state for QID-only assignment editing.

import type { AssignmentItemSummary } from "../../generated/api/AssignmentItemSummary";
import type { Capability } from "../../generated/api/Capability";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { LearnerDisclosurePolicy } from "../../generated/api/LearnerDisclosurePolicy";
import type {
  AssignmentCapabilityViolation,
  AssignmentCreateInput,
  AssignmentEditorInput,
} from "../api/contracts";
import { DEFAULT_MASTERY_TIME_LIMIT_SECONDS } from "../../generated/api/DEFAULT_MASTERY_TIME_LIMIT_SECONDS";
import { MAX_ASSIGNMENT_TIME_LIMIT_SECONDS } from "../../generated/api/MAX_ASSIGNMENT_TIME_LIMIT_SECONDS";
import { normalizeQuestionIdSyntax } from "../question_id";

export interface AssignmentCatalogRow {
  readonly questionId: QuestionId;
  readonly title: string;
  readonly backend: QuestionBackend;
}

export interface AssignmentEditorDraft {
  readonly id: string;
  readonly courseId: string;
  readonly title: string;
  readonly items: ReadonlyArray<AssignmentItemSummary>;
  readonly policies: AssignmentEditorInput["policies"];
  readonly disclosurePolicy: LearnerDisclosurePolicy;
  readonly assignmentTiming: AssignmentEditorInput["assignmentTiming"];
  readonly revision: string;
}

export interface TimeLimitValidation {
  readonly seconds: number | null;
  readonly error: string | null;
}

/** Converts a deliberately typed whole-run duration without rounding it. */
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
  if (numerator === 0n)
    return { seconds: null, error: "Enter a positive number of minutes, such as 15." };
  const denominator = 10n ** BigInt(fraction.length);
  const secondsNumerator = numerator * 60n;
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
  return String((seconds ?? DEFAULT_MASTERY_TIME_LIMIT_SECONDS) / 60);
}

export function moveAssignmentItem(
  draft: AssignmentEditorDraft,
  itemId: string,
  direction: -1 | 1,
): AssignmentEditorDraft {
  const index = draft.items.findIndex((item) => item.id === itemId);
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= draft.items.length) return draft;
  const items = [...draft.items];
  const current = items[index];
  const adjacent = items[nextIndex];
  if (current === undefined || adjacent === undefined) return draft;
  items[index] = { ...adjacent, position: index };
  items[nextIndex] = { ...current, position: nextIndex };
  return { ...draft, items };
}

export function createMasteryAssignmentDraft(courseId: string): AssignmentEditorDraft {
  return {
    id: "",
    courseId,
    title: "",
    items: [],
    policies: {
      completion: { kind: "allCorrect" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
    disclosurePolicy: {
      score: "afterSubmit",
      perItemCorrectness: "afterSubmit",
      feedbackText: "afterSubmit",
      solution: "afterSubmit",
      classStatistics: "never",
    },
    assignmentTiming: { timeLimitSeconds: DEFAULT_MASTERY_TIME_LIMIT_SECONDS },
    revision: "",
  };
}

export function assignmentInput(draft: AssignmentEditorDraft): AssignmentEditorInput {
  return {
    title: draft.title,
    items: draft.items.map(
      ({ id, questionId, position, pointsPossible, deliveryState, scoringMode }) => ({
        id,
        questionId,
        position,
        pointsPossible,
        deliveryState,
        scoringMode,
      }),
    ),
    policies: draft.policies,
    disclosurePolicy: draft.disclosurePolicy,
    assignmentTiming: draft.assignmentTiming,
  };
}

export function assignmentCreateInput(draft: AssignmentEditorDraft): AssignmentCreateInput {
  return {
    title: draft.title,
    questionIds: draft.items.map((item) => item.questionId),
    policies: draft.policies,
    disclosurePolicy: draft.disclosurePolicy,
    assignmentTiming: draft.assignmentTiming,
  };
}

export function parseExactProblemDisplayReferences(value: string): ReadonlyArray<string> {
  const values = value
    .split(/[\n,]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
  if (values.length === 0) throw new Error("Paste at least one Question ID, such as 7K3-M9QP.");
  const normalized = values.map((entry) => normalizeQuestionIdSyntax(entry));
  if (normalized.some((entry) => entry === null))
    throw new Error("Use canonical Question IDs such as 7K3-M9QP.");
  const ids = normalized as string[];
  if (new Set(ids).size !== ids.length)
    throw new Error("Each Question ID can appear once in this operation.");
  return ids;
}

export function capabilityLabel(capability: Capability): string {
  return capability.replace(/([A-Z])/gu, " $1").toLowerCase();
}
export function assignmentProblemLabel(row: AssignmentCatalogRow | AssignmentItemSummary): string {
  return row.questionId;
}
export function questionBackendLabel(backend: QuestionBackend): string {
  return { native: "PLE native", webwork: "WeBWorK", qti: "QTI", h5p: "H5P", imathas: "iMathAS" }[
    backend
  ];
}
export function violationMatchesQuestion(
  violation: AssignmentCapabilityViolation,
  questionId: string,
): boolean {
  return violation.questionId === questionId;
}
