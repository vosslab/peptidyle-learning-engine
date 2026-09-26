// Pure current-Assessment content helpers for the Questions workspace.

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type {
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInput,
} from "../../api/assessment_release";

export type QuestionEditDirtyEvent =
  "title" | "move" | "sort" | "remove" | "add" | "saveSucceeded" | "saveFailed";

/** Keeps the leave guard active until the current structural edit was persisted successfully. */
export function nextQuestionEditDirty(current: boolean, event: QuestionEditDirtyEvent): boolean {
  if (event === "saveSucceeded") return false;
  if (event === "saveFailed") return current;
  return true;
}

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
export function questionRevisionKey(
  publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
): string {
  return `${publishedQuestionRevisionTuple.publishedQuestionId}:${publishedQuestionRevisionTuple.revisionNumber}`;
}

/** Appends one exact Published Revision as a new fixed Entry without altering existing Entry objects. */
export function appendAvailableFixedQuestion(
  entries: ReadonlyArray<AssessmentEntry>,
  publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  entryId: AssessmentEntryId,
): ReadonlyArray<AssessmentEntry> {
  const key = questionRevisionKey(publishedQuestionRevisionTuple);
  if (
    entries.some(
      (entry) =>
        entry.kind === "fixedQuestion" &&
        questionRevisionKey(entry.publishedQuestionRevisionTuple) === key,
    )
  )
    return entries;
  return [
    ...entries,
    {
      kind: "fixedQuestion",
      id: entryId,
      publishedQuestionRevisionTuple,
      pointsPossible: "1",
      availability: "available",
      scoringRule: "normal",
      questionAttemptLimit: { maxAttempts: null },
      questionAttemptTimeLimit: { kind: "unlimited" },
    },
  ];
}

/** Counts only Entries that can contribute Questions to a future Assessment Attempt. */
export function deliveredAssessmentQuestionCount(entries: ReadonlyArray<AssessmentEntry>): number {
  return entries.reduce((count, entry) => {
    if (entry.availability !== "available") return count;
    return count + (entry.kind === "fixedQuestion" ? 1 : entry.selectionCount);
  }, 0);
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

/** Removes only a selected Available Entry; retained unavailable Entries stay read-only. */
export function removeAssessmentEntry(
  entries: ReadonlyArray<AssessmentEntry>,
  index: number,
): ReadonlyArray<AssessmentEntry> {
  if (index < 0 || index >= entries.length) return entries;
  if (entries[index]?.availability !== "available") return entries;
  return entries.filter((_entry, currentIndex) => currentIndex !== index);
}

function cognitiveProcessOrdinal(value: BloomClassificationView["cognitiveProcess"]): number {
  switch (value) {
    case "Remember":
      return 0;
    case "Understand":
      return 1;
    case "Apply":
      return 2;
    case "Analyze":
      return 3;
    case "Evaluate":
      return 4;
    case "Create":
      return 5;
  }
}

function knowledgeDimensionOrdinal(value: BloomClassificationView["knowledgeDimension"]): number {
  switch (value) {
    case "Factual Knowledge":
      return 0;
    case "Conceptual Knowledge":
      return 1;
    case "Procedural Knowledge":
      return 2;
    case "Metacognitive Knowledge":
      return 3;
  }
}

/** Applies guide-order Bloom keys and the immediately prior position as the stable tie-break. */
export function sortAssessmentEntriesByBloom(
  entries: ReadonlyArray<AssessmentEntry>,
  bloomByEntryId: ReadonlyMap<AssessmentEntryId, BloomClassificationView>,
): ReadonlyArray<AssessmentEntry> | undefined {
  const decorated: Array<{
    readonly entry: AssessmentEntry;
    readonly position: number;
    readonly bloom: BloomClassificationView;
  }> = [];
  for (const [position, entry] of entries.entries()) {
    const bloom = bloomByEntryId.get(entry.id);
    if (bloom === undefined) return undefined;
    decorated.push({ entry, position, bloom });
  }
  decorated.sort((left, right) => {
    const cognitive =
      cognitiveProcessOrdinal(left.bloom.cognitiveProcess) -
      cognitiveProcessOrdinal(right.bloom.cognitiveProcess);
    if (cognitive !== 0) return cognitive;
    const knowledge =
      knowledgeDimensionOrdinal(left.bloom.knowledgeDimension) -
      knowledgeDimensionOrdinal(right.bloom.knowledgeDimension);
    return knowledge === 0 ? left.position - right.position : knowledge;
  });
  const sorted = decorated.map((item) => item.entry);
  const unchanged = sorted.every((entry, position) => entry === entries[position]);
  return unchanged ? entries : sorted;
}
