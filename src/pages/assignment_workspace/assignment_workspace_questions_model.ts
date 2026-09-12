// Pure current-Assignment content helpers for the Questions workspace.

import type { AssignmentEntry } from "../../../generated/api/AssignmentEntry";
import type { AssignmentEntryId } from "../../../generated/api/AssignmentEntryId";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { AssignmentQuestionPickerEntry } from "../../api/assignment_release";

/** A stable, exact identity used when comparing pinned Question Revisions. */
export function questionRevisionKey(reference: QuestionRevisionReference): string {
  return `${reference.questionId}:${reference.revisionNumber}`;
}

/** Appends an Available picker row as a new fixed Entry without altering existing Entry objects. */
export function appendAvailableFixedQuestion(
  entries: ReadonlyArray<AssignmentEntry>,
  picker: AssignmentQuestionPickerEntry,
  entryId: AssignmentEntryId,
): ReadonlyArray<AssignmentEntry> {
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
export function moveAssignmentEntry(
  entries: ReadonlyArray<AssignmentEntry>,
  index: number,
  offset: -1 | 1,
): ReadonlyArray<AssignmentEntry> {
  const target = index + offset;
  if (index < 0 || target < 0 || index >= entries.length || target >= entries.length)
    return entries;
  const next = [...entries];
  [next[index], next[target]] = [next[target]!, next[index]!];
  return next;
}

/** Removes only the selected stable Entry; other Entry and Pool Item identities survive. */
export function removeAssignmentEntry(
  entries: ReadonlyArray<AssignmentEntry>,
  index: number,
): ReadonlyArray<AssignmentEntry> {
  if (index < 0 || index >= entries.length) return entries;
  return entries.filter((_entry, currentIndex) => currentIndex !== index);
}
