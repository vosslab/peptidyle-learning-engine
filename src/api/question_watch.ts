// Closed self-only browser contract for one Published Question Watch state.

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";

/** No watcher identity, count, list, activity, or notification is exposed. */
export interface QuestionWatchProjection {
  readonly watching: boolean;
}

/** Active-Instructor self-service Watch boundary for one Published Question. */
export interface QuestionWatchClient {
  readonly getQuestionWatch: (questionId: PublishedQuestionId) => Promise<QuestionWatchProjection>;
  readonly setQuestionWatch: (
    questionId: PublishedQuestionId,
    watching: boolean,
  ) => Promise<QuestionWatchProjection>;
}
