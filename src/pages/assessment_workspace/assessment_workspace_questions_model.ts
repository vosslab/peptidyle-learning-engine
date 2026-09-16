// Pure current-Assessment content helpers for the Questions workspace.

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type {
  AssessmentQuestionPickerEntry,
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInput,
} from "../../api/assessment_release";

/** Builds the closed full-Assessment save payload without dropping unedited fields. */
export function questionSaveInput(
  current: LiveAssessmentWorkspace,
  title: string,
  entries: ReadonlyArray<AssessmentEntry>,
): SaveLiveAssessmentInput {
  return {
    title,
    instructions: current.instructions,
    entries,
    dueAt: current.dueAt,
    availableAt: current.availableAt,
    closesAt: current.closesAt,
    lateWorkRule: current.lateWorkRule,
    assessmentAttemptTimeLimitSeconds: current.assessmentAttemptTimeLimitSeconds,
    attemptLimit: current.attemptLimit,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule: current.studentFeedbackReleaseRule,
  };
}

/** Parses the exact bounded decimal grammar already enforced by the Assessment API. */
export function assessmentPointValueDraft(value: string): AssessmentPointValue | undefined {
  if (!/^[0-9]{1,10}(?:\.[0-9]{0,4})?$/u.test(value)) return undefined;
  const whole = BigInt(value.split(".")[0] ?? "0");
  return whole <= 1_000_000_000n ? value : undefined;
}

/** Replaces only fixed-Question point values while retaining every other Entry field and order. */
export function withFixedQuestionPointValues(
  entries: ReadonlyArray<AssessmentEntry>,
  pointsByEntryId: Readonly<Record<string, AssessmentPointValue>>,
): ReadonlyArray<AssessmentEntry> {
  return entries.map((entry) => {
    if (entry.kind !== "fixedQuestion") return entry;
    const pointsPossible = pointsByEntryId[entry.id];
    return pointsPossible === undefined || pointsPossible === entry.pointsPossible
      ? entry
      : { ...entry, pointsPossible };
  });
}

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
