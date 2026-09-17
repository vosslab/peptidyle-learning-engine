// Closed Pool endorsement and actor-private subscription contracts.

import type { QuestionId } from "../../generated/api/QuestionId";

/** One immutable vetted public name, never an Account or Profile reference. */
export interface QuestionPoolStarredInstructor {
  readonly displayName: string;
}

/** Public endorsement facts available only to an authorized vetted Instructor. */
export interface QuestionPoolStarProjection {
  readonly starCount: number;
  readonly viewerHasStarred: boolean;
  readonly starredInstructors: ReadonlyArray<QuestionPoolStarredInstructor>;
}

/** The current Instructor's state only; no watcher list, count or notification. */
export interface QuestionPoolWatchProjection {
  readonly watching: boolean;
}

/** Explicit own-state operations for one stable public Pool identity. */
export interface QuestionPoolStewardshipClient {
  readonly getQuestionPoolStar: (poolId: QuestionId) => Promise<QuestionPoolStarProjection>;
  readonly setQuestionPoolStar: (
    poolId: QuestionId,
    starred: boolean,
  ) => Promise<QuestionPoolStarProjection>;
  readonly getQuestionPoolWatch: (poolId: QuestionId) => Promise<QuestionPoolWatchProjection>;
  readonly setQuestionPoolWatch: (
    poolId: QuestionId,
    watching: boolean,
  ) => Promise<QuestionPoolWatchProjection>;
}
