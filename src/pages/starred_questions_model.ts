// starred_questions_model.ts - the current Instructor's personal Star collection.

import type { ApiClient } from "../api/client";
import type { StarredQuestionPage } from "../api/question_star";
import type { RecordPageSize } from "../components/record_list/record_page_controls";

export interface StarredQuestionsPosition {
  readonly pageSize: RecordPageSize;
  readonly inputCursor: string | null;
  readonly previousCursors: ReadonlyArray<string | null>;
}

export const FIRST_STARRED_QUESTIONS_POSITION: StarredQuestionsPosition = {
  pageSize: 50,
  inputCursor: null,
  previousCursors: [],
};

/** Loads the signed-in Instructor's Starred Questions, newest first. */
export async function loadStarredQuestions(
  client: Pick<ApiClient, "listStarredQuestions">,
  position: StarredQuestionsPosition = FIRST_STARRED_QUESTIONS_POSITION,
): Promise<StarredQuestionPage> {
  return client.listStarredQuestions(position.inputCursor, position.pageSize);
}

export function starredQuestionsNextPosition(
  position: StarredQuestionsPosition,
  nextCursor: string,
): StarredQuestionsPosition {
  return {
    pageSize: position.pageSize,
    inputCursor: nextCursor,
    previousCursors: [...position.previousCursors, position.inputCursor],
  };
}

export function starredQuestionsPreviousPosition(
  position: StarredQuestionsPosition,
): StarredQuestionsPosition | null {
  if (position.previousCursors.length === 0) return null;
  const previousCursors = position.previousCursors.slice(0, -1);
  return {
    pageSize: position.pageSize,
    inputCursor: position.previousCursors[position.previousCursors.length - 1] ?? null,
    previousCursors,
  };
}

export function starredQuestionsPageSizePosition(
  pageSize: RecordPageSize,
): StarredQuestionsPosition {
  return { pageSize, inputCursor: null, previousCursors: [] };
}
