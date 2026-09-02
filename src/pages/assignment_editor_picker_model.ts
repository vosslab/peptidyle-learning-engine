// assignment_editor_picker_model.ts - bounded selection limits for assignment picker destinations.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_QUESTION_POOL_ITEMS } from "../../generated/api/MAX_ASSIGNMENT_QUESTION_POOL_ITEMS";

import type { AssignmentEditorState } from "./assignment_editor_model";

export type AssignmentPickerIntent =
  { readonly kind: "fixedQuestion" } | { readonly kind: "pool"; readonly entryIndex: number };

/** Computes the destination-specific capacity before a shared picker opens. */
export function assignmentPickerMaximum(
  draft: AssignmentEditorState,
  intent: AssignmentPickerIntent,
): number {
  if (intent.kind === "fixedQuestion") {
    return Math.max(0, MAX_ASSIGNMENT_ORDERED_ENTRIES - draft.entries.length);
  }
  const entry = draft.entries[intent.entryIndex];
  if (entry === undefined || entry.kind !== "questionPool") return 0;
  const usedQuestionPoolItems = draft.entries.reduce(
    (count, item) => count + (item.kind === "questionPool" ? item.items.length : 0),
    0,
  );
  const poolRemaining = MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY - entry.items.length;
  const assignmentRemaining = MAX_ASSIGNMENT_QUESTION_POOL_ITEMS - usedQuestionPoolItems;
  return Math.max(0, Math.min(poolRemaining, assignmentRemaining));
}
