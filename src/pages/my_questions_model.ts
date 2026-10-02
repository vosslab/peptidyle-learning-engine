// my_questions_model.ts - the current Instructor's Published Question list request.

import type { ApiClient } from "../api/client";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import {
  decodeQuestionLibraryBrowsePage,
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  FIRST_QUESTION_LIBRARY_BROWSE_POSITION,
  type QuestionLibraryBrowsePage,
  type QuestionLibraryPageSize,
} from "./library_page_model";

/** Cursor sequence for one authored-question page size. The first page uses a null cursor. */
export interface MyQuestionsPosition {
  readonly pageSize: QuestionLibraryPageSize;
  readonly inputCursor: string | null;
  readonly previousCursors: ReadonlyArray<string | null>;
}

export const FIRST_MY_QUESTIONS_POSITION: MyQuestionsPosition = {
  pageSize: FIRST_QUESTION_LIBRARY_BROWSE_POSITION.pageSize,
  inputCursor: null,
  previousCursors: [],
};

export interface MyQuestionsPageRequest {
  readonly cursor: string | null;
  readonly pageSize: QuestionLibraryPageSize;
}

/**
 * Loads one page of Published Questions authored by the signed-in Account.
 * ASVS 8.2.2: authoredByCurrentAccount is a closed flag the server binds to the session.
 * ASVS 2.2.1: page size is 50, 100, or 250, and the cursor is only the server continuation.
 */
export async function loadMyQuestions(
  client: Pick<ApiClient, "searchQuestionLibrary">,
  request: MyQuestionsPageRequest = {
    cursor: FIRST_MY_QUESTIONS_POSITION.inputCursor,
    pageSize: FIRST_MY_QUESTIONS_POSITION.pageSize,
  },
): Promise<QuestionLibraryBrowsePage> {
  const repository = createQuestionLibraryRepository(
    client as ApiClient,
    "authoredByCurrentAccount",
  );
  return decodeQuestionLibraryBrowsePage(
    await repository.search(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, request.cursor, request.pageSize),
  );
}

/** Moves to the server-issued next page and remembers the current cursor. */
export function myQuestionsNextPosition(
  position: MyQuestionsPosition,
  nextCursor: string,
): MyQuestionsPosition {
  return {
    pageSize: position.pageSize,
    inputCursor: nextCursor,
    previousCursors: [...position.previousCursors, position.inputCursor],
  };
}

/** Returns the previous server page, or null when this is already the first page. */
export function myQuestionsPreviousPosition(
  position: MyQuestionsPosition,
): MyQuestionsPosition | null {
  if (position.previousCursors.length === 0) return null;
  const previousCursors = position.previousCursors.slice(0, -1);
  const inputCursor = position.previousCursors[position.previousCursors.length - 1] ?? null;
  return {
    pageSize: position.pageSize,
    inputCursor,
    previousCursors,
  };
}

/** Starts a new cursor sequence when the Instructor changes the page size. */
export function myQuestionsPageSizePosition(
  pageSize: QuestionLibraryPageSize,
): MyQuestionsPosition {
  return {
    pageSize,
    inputCursor: null,
    previousCursors: [],
  };
}
