import type { LiveAssessmentPreviousAttempt } from "../api/assessment_attempt_issuance";
import type { RecordField } from "../components/data_sheet/data_sheet_fields";

export type PreviousAttemptSortKey = "attemptNumber" | "state" | "score";
export type PreviousAttemptFilterKey = "state";

export const previousAttemptSearchFields: ReadonlyArray<
  RecordField<LiveAssessmentPreviousAttempt, PreviousAttemptSortKey, PreviousAttemptFilterKey>
> = [
  {
    id: "attemptNumber",
    label: "Attempt",
    role: "identity",
    sortKey: "attemptNumber",
    value: (attempt) => `Attempt ${attempt.attemptNumber}`,
    defaultColumn: { priority: 0, width: "12rem" },
  },
  {
    id: "state",
    label: "State",
    sortKey: "state",
    filterKey: "state",
    value: (attempt) => (attempt.state === "submitted" ? "Submitted" : "Closed"),
    defaultColumn: { priority: 1, width: "10rem" },
  },
  {
    id: "score",
    label: "Points earned",
    sortKey: "score",
    value: (attempt) => attempt.score?.pointsEarned ?? null,
    defaultColumn: { priority: 2, width: "10rem" },
  },
];
