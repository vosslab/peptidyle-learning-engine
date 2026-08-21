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
  readonly revision: string;
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
  };
}

export function assignmentCreateInput(draft: AssignmentEditorDraft): AssignmentCreateInput {
  return {
    title: draft.title,
    questionIds: draft.items.map((item) => item.questionId),
    policies: draft.policies,
    disclosurePolicy: draft.disclosurePolicy,
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
