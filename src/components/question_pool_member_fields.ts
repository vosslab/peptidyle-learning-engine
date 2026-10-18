import type { RecordField } from "./data_sheet/data_sheet_fields";
import {
  compareQuestionPoolMemberRows,
  type QuestionPoolMemberSortEntry,
} from "./question_pool_member_sort";

export type PoolMemberSortKey = "title" | "questionId" | "revision" | "license";
export type PoolMemberFilterKey = "license";

/** The same typed columns and whole-array comparator serve Pool detail and draft editing. */
export function questionPoolMemberFields<Row extends QuestionPoolMemberSortEntry>(): ReadonlyArray<
  RecordField<Row, PoolMemberSortKey, PoolMemberFilterKey>
> {
  return [
    {
      id: "title",
      label: "Question Title",
      role: "identity",
      sortKey: "title",
      value: (row) => row.questionTitle,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "questionId",
      label: "Published Question ID",
      sortKey: "questionId",
      value: (row) => row.publishedQuestionRevisionTuple.publishedQuestionId,
      defaultColumn: { priority: 1, width: "14rem" },
    },
    {
      id: "revision",
      label: "Revision",
      sortKey: "revision",
      value: (row) => row.publishedQuestionRevisionTuple.revisionNumber,
      defaultColumn: { priority: 2, width: "6rem" },
    },
    {
      id: "license",
      label: "Question License",
      sortKey: "license",
      filterKey: "license",
      value: (row) => row.questionLicense ?? "Unavailable",
      defaultColumn: { priority: 3, width: "11rem" },
    },
  ];
}

/** The local adapter applies direction after this existing Pool comparison policy. */
export function comparePoolMemberFields<Row extends QuestionPoolMemberSortEntry>(
  key: PoolMemberSortKey,
  left: Row,
  right: Row,
): number {
  const sort = {
    title: "title-ascending",
    questionId: "question-id-ascending",
    revision: "revision-ascending",
    license: "license-ascending",
  } as const;
  return compareQuestionPoolMemberRows(left, right, sort[key]);
}

export function poolMemberRowId(row: QuestionPoolMemberSortEntry): string {
  const tuple = row.publishedQuestionRevisionTuple;
  return `${tuple.publishedQuestionId}:${tuple.revisionNumber}`;
}
