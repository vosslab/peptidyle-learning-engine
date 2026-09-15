// Closed self-only browser contract for one Published Question Watch state.

import type { QuestionId } from "../../generated/api/QuestionId";

/** No watcher identity, count, list, activity, or notification is exposed. */
export interface QuestionWatchProjection {
  readonly watching: boolean;
}

/** Active-Instructor self-service Watch boundary for one Published Question. */
export interface QuestionWatchClient {
  readonly getQuestionWatch: (questionId: QuestionId) => Promise<QuestionWatchProjection>;
  readonly setQuestionWatch: (
    questionId: QuestionId,
    watching: boolean,
  ) => Promise<QuestionWatchProjection>;
}
