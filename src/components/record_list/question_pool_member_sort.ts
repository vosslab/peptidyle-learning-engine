import type { QuestionPoolMemberView } from "../../../generated/api/QuestionPoolMemberView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";

export const questionPoolMemberSortOptions = [
  { value: "as-loaded", label: "As loaded" },
  { value: "title-ascending", label: "Title (A to Z)" },
  { value: "title-descending", label: "Title (Z to A)" },
  { value: "question-id-ascending", label: "Question ID (low to high)" },
  { value: "question-id-descending", label: "Question ID (high to low)" },
  { value: "revision-ascending", label: "Revision (low to high)" },
  { value: "revision-descending", label: "Revision (high to low)" },
  { value: "license-ascending", label: "Question License (A to Z)" },
  { value: "license-descending", label: "Question License (Z to A)" },
] as const;

export type QuestionPoolMemberSort = (typeof questionPoolMemberSortOptions)[number]["value"];

export interface QuestionPoolMemberSortEntry {
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly questionLicense?: string | null;
}

function compareText(left: string, right: string): number {
  return left.localeCompare(right, "en", { numeric: true, sensitivity: "base" });
}

function compareMembers(
  left: QuestionPoolMemberSortEntry,
  right: QuestionPoolMemberSortEntry,
  sort: QuestionPoolMemberSort,
): number {
  const leftRevision = left.publishedQuestionRevisionTuple.revisionNumber;
  const rightRevision = right.publishedQuestionRevisionTuple.revisionNumber;
  const comparison = ((): number => {
    switch (sort) {
      case "title-ascending":
      case "title-descending":
        return compareText(left.questionTitle, right.questionTitle);
      case "question-id-ascending":
      case "question-id-descending":
        return compareText(
          left.publishedQuestionRevisionTuple.publishedQuestionId,
          right.publishedQuestionRevisionTuple.publishedQuestionId,
        );
      case "revision-ascending":
      case "revision-descending":
        return leftRevision - rightRevision;
      case "license-ascending":
      case "license-descending":
        return compareText(left.questionLicense ?? "", right.questionLicense ?? "");
      case "as-loaded":
        return 0;
    }
  })();
  return sort.endsWith("descending") ? -comparison : comparison;
}

/** Return a sorted display copy while preserving the caller's row array. */
export function sortQuestionPoolMemberRows<T extends QuestionPoolMemberSortEntry>(
  members: ReadonlyArray<T>,
  sort: QuestionPoolMemberSort,
): ReadonlyArray<T> {
  if (sort === "as-loaded") return [...members];
  return members
    .map((member, originalIndex) => ({ member, originalIndex }))
    .sort((left, right) => {
      const comparison = compareMembers(left.member, right.member, sort);
      return comparison === 0 ? left.originalIndex - right.originalIndex : comparison;
    })
    .map(({ member }) => member);
}

/** Sort a display copy of Pool API rows by their current answer-free metadata. */
export function sortQuestionPoolMembers(
  members: ReadonlyArray<QuestionPoolMemberView>,
  sort: QuestionPoolMemberSort,
): ReadonlyArray<QuestionPoolMemberView> {
  const rows = members.map((member) => ({
    member,
    publishedQuestionRevisionTuple: member.publishedQuestionRevisionTuple,
    questionTitle: member.question.question_library.summary.metadata.questionTitle,
    questionLicense: member.question.question_library.summary.metadata.questionLicense,
  }));
  return sortQuestionPoolMemberRows(rows, sort).map(({ member }) => member);
}
