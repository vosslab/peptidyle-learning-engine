import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { QuestionPoolEditNumber } from "../../../generated/api/QuestionPoolEditNumber";
import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { LibrarySearchRow } from "../../pages/library_page_model";

export type AssessmentContentPickerSelection =
  | {
      readonly kind: "questions";
      readonly questions: ReadonlyArray<{
        readonly title: string;
        readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
      }>;
    }
  | {
      readonly kind: "pool";
      readonly title: string;
      readonly questionPoolId: QuestionPoolId;
      readonly questionPoolEditNumber: QuestionPoolEditNumber;
      readonly memberCount: number;
    };

function poolSelectionFromRow(
  row: Extract<LibrarySearchRow, { readonly kind: "pool" }>,
): Extract<AssessmentContentPickerSelection, { readonly kind: "pool" }> {
  return {
    kind: "pool",
    title: row.title,
    questionPoolId: row.displayId,
    questionPoolEditNumber: row.questionPoolEditNumber,
    memberCount: row.memberCount,
  };
}

/** Converts retained shared-search selections into the two Assessment workflow inputs. */
export function assessmentContentPickerSelection(
  rows: ReadonlyArray<LibrarySearchRow>,
): AssessmentContentPickerSelection | undefined {
  const pool = rows.find(
    (row): row is Extract<LibrarySearchRow, { readonly kind: "pool" }> => row.kind === "pool",
  );
  if (pool !== undefined) return poolSelectionFromRow(pool);
  const questions = rows
    .filter(
      (row): row is Extract<LibrarySearchRow, { readonly kind: "question" }> =>
        row.kind === "question",
    )
    .map((row) => ({
      title: row.questionTitle,
      publishedQuestionRevisionTuple: row.publishedQuestionRevisionTuple,
    }));
  return questions.length === 0 ? undefined : { kind: "questions", questions };
}
