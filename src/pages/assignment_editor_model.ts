// assignment_editor_model.ts - local state helpers for revisioned assignment editing.

import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { Capability } from "../../generated/api/Capability";
import type { AssignmentCapabilityViolation, AssignmentEditorInput } from "../api/contracts";

export interface AssignmentCatalogRow {
  readonly reference: ProblemVersionRef;
  readonly title: string;
}

export interface AssignmentEditorDraft extends AssignmentEditorInput {
  readonly id: string;
  readonly courseId: string;
  readonly revision: string;
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
