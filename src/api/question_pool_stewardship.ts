// Closed Pool endorsement and actor-private subscription contracts.

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";

/** One active Instructor shown in an Instructor-only Star list. */
export interface QuestionPoolStarredInstructor {
  readonly displayName: string;
  readonly accountId: string;
}

/** Public endorsement facts available only to an authorized active Instructor. */
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
  readonly getQuestionPoolStar: (poolId: QuestionPoolId) => Promise<QuestionPoolStarProjection>;
  readonly setQuestionPoolStar: (
    poolId: QuestionPoolId,
    starred: boolean,
  ) => Promise<QuestionPoolStarProjection>;
  readonly getQuestionPoolWatch: (poolId: QuestionPoolId) => Promise<QuestionPoolWatchProjection>;
  readonly setQuestionPoolWatch: (
    poolId: QuestionPoolId,
    watching: boolean,
  ) => Promise<QuestionPoolWatchProjection>;
}
