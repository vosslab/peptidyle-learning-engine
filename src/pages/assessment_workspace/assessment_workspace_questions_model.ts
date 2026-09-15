// Pure current-Assessment content helpers for the Questions workspace.

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { AssessmentQuestionPickerEntry } from "../../api/assessment_release";

/** A stable, exact identity used when comparing pinned Question Revisions. */
export function questionRevisionKey(reference: QuestionRevisionReference): string {
  return `${reference.questionId}:${reference.revisionNumber}`;
}

/** Appends an Available picker row as a new fixed Entry without altering existing Entry objects. */
export function appendAvailableFixedQuestion(
  entries: ReadonlyArray<AssessmentEntry>,
  picker: AssessmentQuestionPickerEntry,
  entryId: AssessmentEntryId,
): ReadonlyArray<AssessmentEntry> {
  const key = questionRevisionKey(picker.reference);
  if (
    entries.some(
      (entry) => entry.kind === "fixedQuestion" && questionRevisionKey(entry.reference) === key,
    )
  )
    return entries;
  return [
    ...entries,
    {
      kind: "fixedQuestion",
      id: entryId,
      reference: picker.reference,
      pointsPossible: "1",
      availability: "available",
      scoringRule: "normal",
      questionAttemptLimit: { maxAttempts: null },
      questionAttemptTimeLimit: { kind: "unlimited" },
    },
  ];
}

/** Reorders Entry identities while retaining every Entry and Pool Item unchanged. */
export function moveAssessmentEntry(
  entries: ReadonlyArray<AssessmentEntry>,
  index: number,
  offset: -1 | 1,
): ReadonlyArray<AssessmentEntry> {
  const target = index + offset;
  if (index < 0 || target < 0 || index >= entries.length || target >= entries.length)
    return entries;
  const next = [...entries];
  [next[index], next[target]] = [next[target]!, next[index]!];
  return next;
}

/** Removes only the selected stable Entry; other Entry and Pool Item identities survive. */
export function removeAssessmentEntry(
  entries: ReadonlyArray<AssessmentEntry>,
  index: number,
): ReadonlyArray<AssessmentEntry> {
  if (index < 0 || index >= entries.length) return entries;
  return entries.filter((_entry, currentIndex) => currentIndex !== index);
}
