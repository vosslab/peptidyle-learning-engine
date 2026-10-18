import type { RecordField } from "../components/data_sheet/record_fields";

export type DraftSearchSortKey = "title" | "editNumber";
export type DraftSearchFilterKey = "origin";

export type DraftSearchRow = {
  readonly draftQuestionId: string;
  readonly draftQuestionEditNumber: string;
  readonly questionTitle: string;
  readonly questionDescription: string;
  readonly parentPublishedQuestionRevisionTuple: unknown;
};

function origin(row: DraftSearchRow): string {
  return row.parentPublishedQuestionRevisionTuple === null
    ? "New Question"
    : "Published Question revision";
}

export const questionDraftSearchFields: ReadonlyArray<
  RecordField<DraftSearchRow, DraftSearchSortKey, DraftSearchFilterKey>
> = [
  {
    id: "title",
    label: "Title",
    role: "identity",
    sortKey: "title",
    value: (row) => row.questionTitle,
    defaultColumn: { priority: 0, width: "18rem" },
  },
  {
    id: "origin",
    label: "Origin",
    filterKey: "origin",
    value: origin,
    defaultColumn: { priority: 1, width: "12rem" },
  },
  {
    id: "editNumber",
    label: "Edit Number",
    sortKey: "editNumber",
    value: (row) => row.draftQuestionEditNumber,
    defaultColumn: { priority: 2, width: "8rem" },
  },
  {
    id: "description",
    label: "Description",
    value: (row) => row.questionDescription,
    defaultColumn: { priority: 3, width: "18rem", visible: false },
  },
];

/** The API validates decimal bigint strings; compare without JavaScript number rounding. */
export function compareDraftEditNumbers(left: DraftSearchRow, right: DraftSearchRow): number {
  const first = left.draftQuestionEditNumber;
  const second = right.draftQuestionEditNumber;
  return first.length === second.length
    ? first.localeCompare(second)
    : first.length - second.length;
}
