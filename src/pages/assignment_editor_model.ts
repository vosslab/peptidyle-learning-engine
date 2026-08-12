// assignment_editor_model.ts - local state helpers for revisioned assignment editing.

import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { Capability } from "../../generated/api/Capability";
import type { ProblemPublicId } from "../../generated/api/ProblemPublicId";
import type { ProblemVersionNumber } from "../../generated/api/ProblemVersionNumber";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { AssignmentCapabilityViolation, AssignmentEditorInput } from "../api/contracts";

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
  };
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
