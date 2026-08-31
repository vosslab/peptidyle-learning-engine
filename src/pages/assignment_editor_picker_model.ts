// assignment_editor_picker_model.ts - bounded selection limits for assignment picker destinations.

import { MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL } from "../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES } from "../../generated/api/MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES";

import type { AssignmentEditorDraft } from "./assignment_editor_model";

export type AssignmentPickerIntent =
  | { readonly kind: "fixedQuestion" }
  | { readonly kind: "pool"; readonly entryIndex: number }
  | { readonly kind: "replacement"; readonly itemId: string };

/** Computes the destination-specific capacity before a shared picker opens. */
export function assignmentPickerMaximum(
  draft: AssignmentEditorDraft,
  intent: AssignmentPickerIntent,
): number {
  if (intent.kind === "replacement") return 1;
  if (intent.kind === "fixedQuestion") {
    return Math.max(0, MAX_ASSIGNMENT_ORDERED_ENTRIES - draft.entries.length);
  }
  const entry = draft.entries[intent.entryIndex];
  if (entry === undefined || entry.kind !== "questionPool") return 0;
  const usedCandidates = draft.entries.reduce(
    (count, item) => count + (item.kind === "questionPool" ? item.candidates.length : 0),
    0,
  );
  const poolRemaining = MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL - entry.candidates.length;
  const assignmentRemaining = MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES - usedCandidates;
  return Math.max(0, Math.min(poolRemaining, assignmentRemaining));
}
