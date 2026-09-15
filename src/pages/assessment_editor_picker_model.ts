// assessment_editor_picker_model.ts - bounded selection limits for assessment picker destinations.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_ASSESSMENT_QUESTION_POOL_ITEMS } from "../../generated/api/MAX_ASSESSMENT_QUESTION_POOL_ITEMS";

import type { AssessmentEditorState } from "./assessment_editor_model";

export type AssessmentPickerIntent =
  { readonly kind: "fixedQuestion" } | { readonly kind: "pool"; readonly entryIndex: number };

/** Computes the destination-specific capacity before a shared picker opens. */
export function assessmentPickerMaximum(
  draft: AssessmentEditorState,
  intent: AssessmentPickerIntent,
): number {
  if (intent.kind === "fixedQuestion") {
    return Math.max(0, MAX_ASSESSMENT_ORDERED_ENTRIES - draft.entries.length);
  }
  const entry = draft.entries[intent.entryIndex];
  if (entry === undefined || entry.kind !== "questionPool") return 0;
  const usedQuestionPoolItems = draft.entries.reduce(
    (count, item) => count + (item.kind === "questionPool" ? item.items.length : 0),
    0,
  );
  const poolRemaining = MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY - entry.items.length;
  const assessmentRemaining = MAX_ASSESSMENT_QUESTION_POOL_ITEMS - usedQuestionPoolItems;
  return Math.max(0, Math.min(poolRemaining, assessmentRemaining));
}
