// Stable source binding for creating one Question Pool from a Published Question.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";
import { decodeQuestionLibraryBrowsePage } from "../pages/library_page_model";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import { validateCanonicalQuestionIdSyntax } from "../question_id";
import type {
  QuestionPickerSelection,
  QuestionPickerSourceRepository,
} from "../features/question_picker/question_picker_model";

export interface QuestionPoolStartingQuestion {
  readonly questionRevisionTuple: QuestionRevisionTuple;
  readonly questionTitle: string;
  readonly disciplineName: string;
  readonly subjectName: string;
}

function canonicalQuestionId(value: string): QuestionId {
  const questionId = validateCanonicalQuestionIdSyntax(value);
  if (questionId === null || questionId !== value) {
    throw new Error("The selected Question is no longer a canonical Published Question.");
  }
  return questionId;
}

function exactStartingRevision(
  startingQuestion: QuestionPoolStartingQuestion,
): QuestionRevisionTuple {
  const questionId = canonicalQuestionId(startingQuestion.questionRevisionTuple.questionId);
  const revisionNumber = startingQuestion.questionRevisionTuple.revisionNumber;
  if (!Number.isSafeInteger(revisionNumber) || revisionNumber < 1) {
    throw new Error("The starting Question Revision is no longer valid.");
  }
  return { questionId, revisionNumber };
}

async function latestSelectedRevisions(
  selection: QuestionPickerSelection,
  getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>,
  startingQuestion?: QuestionPoolStartingQuestion,
): Promise<ReadonlyArray<QuestionRevisionTuple>> {
  return await Promise.all(
    selection.questions.map(async (selected) => {
      const questionId = canonicalQuestionId(selected.questionId);
      if (questionId === startingQuestion?.questionRevisionTuple.questionId) {
        throw new Error("The starting Question is already fixed at the first Pool position.");
      }
      const detail = await getQuestionDetails(questionId);
      const questionRevisionTuple = detail.summary.questionRevisionTuple;
      if (questionRevisionTuple.questionId !== questionId) {
        throw new Error("The selected Question did not resolve to its current published Revision.");
      }
      if (
        startingQuestion !== undefined &&
        (detail.disciplineName !== startingQuestion.disciplineName ||
          detail.subjectName !== startingQuestion.subjectName)
      ) {
        throw new Error("Every Pool Question must share the starting Discipline and Subject.");
      }
      return questionRevisionTuple;
    }),
  );
}

/** Pins the exact starting Revision first, then resolves additional current selections in order. */
export async function questionPoolMemberTuples(
  selection: QuestionPickerSelection,
  getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>,
  startingQuestion?: QuestionPoolStartingQuestion,
): Promise<ReadonlyArray<QuestionRevisionTuple>> {
  const additional = await latestSelectedRevisions(selection, getQuestionDetails, startingQuestion);
  if (startingQuestion === undefined) return additional;
  return [exactStartingRevision(startingQuestion), ...additional];
}

/**
 * Reuses the Question Library picker while fixing its source-bound Subject and
 * retaining only rows from the starting Question's Discipline.
 */
export function questionPoolSourcePickerRepository(
  library: QuestionLibraryBrowseRepository,
  startingQuestion: QuestionPoolStartingQuestion,
): QuestionPickerSourceRepository {
  return {
    async search(request): Promise<unknown> {
      if (request.source.kind !== "library" && request.source.kind !== "sharedLibrary") {
        throw new Error("Choose the Question Library source for this Pool.");
      }
      const seenCursors = new Set<string>();
      let cursor = request.cursor;
      for (;;) {
        if (cursor !== null) {
          if (seenCursors.has(cursor)) {
            throw new Error("Question Library paging repeated a continuation.");
          }
          seenCursors.add(cursor);
        }
        const raw = await library.search(
          { ...request.query, subjects: [startingQuestion.subjectName] },
          cursor,
        );
        const page = decodeQuestionLibraryBrowsePage(raw);
        const items = page.items.filter(
          (row) =>
            row.disciplineName === startingQuestion.disciplineName &&
            row.displayId !== startingQuestion.questionRevisionTuple.questionId,
        );
        if (items.length > 0 || page.nextCursor === null) {
          return { ...page, items };
        }
        cursor = page.nextCursor;
      }
    },
  };
}
