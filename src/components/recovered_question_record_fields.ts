import type { RecoveredQuestion } from "../api/course_student_work_recovery";
import type { RecordField } from "./data_sheet/record_fields";
import type { RecordContent } from "./data_sheet/record_content";
import { labeledQuestionRecognition } from "../features/recognition_label";

export type RecoveredQuestionRecord = RecoveredQuestion & { readonly questionTitle: string };
type SortKey = "position" | "revision";
type FilterKey = "availability";

export function recoveredQuestionTitle(question: RecoveredQuestionRecord): string {
  return `Issued Question ${question.issuedPosition + 1}: ${question.questionTitle}`;
}

export function recoveredQuestionIdentifier(question: RecoveredQuestionRecord): string {
  const revision = question.publishedQuestionRevisionTuple;
  return labeledQuestionRecognition(
    question.questionTitle,
    revision.publishedQuestionId,
    revision.revisionNumber,
  ).identifier;
}

export function recoveredEvidenceAvailability(question: RecoveredQuestionRecord): string {
  return question.unavailableEvidence.length > 0
    ? "Some evidence unavailable"
    : "No unavailable evidence reported";
}

export const recoveredQuestionRecordFields: ReadonlyArray<
  RecordField<RecoveredQuestionRecord, SortKey, FilterKey>
> = [
  {
    id: "question",
    label: "Issued Question",
    role: "identity",
    sortKey: "position",
    value: recoveredQuestionTitle,
    defaultColumn: { priority: 0, width: "24rem" },
  },
  {
    id: "question_id",
    label: "Question ID",
    value: (question) => question.publishedQuestionRevisionTuple.publishedQuestionId,
    defaultColumn: { priority: 1, width: "14rem" },
  },
  {
    id: "revision",
    label: "Revision",
    sortKey: "revision",
    value: (question) => question.publishedQuestionRevisionTuple.revisionNumber,
    defaultColumn: { priority: 2, width: "8rem" },
  },
  {
    id: "availability",
    label: "Evidence availability",
    filterKey: "availability",
    value: recoveredEvidenceAvailability,
    defaultColumn: { priority: 3, width: "18rem" },
  },
];

export function compareRecoveredQuestionRecords(
  key: SortKey,
  left: RecoveredQuestionRecord,
  right: RecoveredQuestionRecord,
): number | undefined {
  return key === "position" ? left.issuedPosition - right.issuedPosition : undefined;
}

export function recoveredQuestionRecordKeyword(question: RecoveredQuestionRecord): string {
  return `${recoveredQuestionTitle(question)} ${recoveredQuestionIdentifier(question)} ${recoveredEvidenceAvailability(question)} ${question.unavailableEvidence.join(" ")}`;
}

/** Summaries expose retained identity and availability; evidence stays in its inert disclosures. */
export function recoveredQuestionRecordContent(question: RecoveredQuestionRecord): RecordContent {
  return {
    title: recoveredQuestionTitle(question),
    details: [
      { kind: "text", label: "Exact Revision", value: recoveredQuestionIdentifier(question) },
      {
        kind: "text",
        label: "Evidence availability",
        value: recoveredEvidenceAvailability(question),
      },
    ],
    actions: [],
  };
}
