// Closed browser contract for one Published Question Star projection.

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";

/** One server-projected verified Instructor display name, not an Account identity. */
export interface QuestionStarredInstructor {
  readonly displayName: string;
}

/**
 * Active-Instructor Star facts for one Published Question.
 *
 * The names are the complete server projection. This contract deliberately
 * carries no Profile route, avatar, email, Account ID, Course, or
 * Watch state from which the browser could reconstruct another identity.
 */
export interface QuestionStarProjection {
  readonly starCount: number;
  readonly viewerHasStarred: boolean;
  readonly starredInstructors: ReadonlyArray<QuestionStarredInstructor>;
}

/** Active-Instructor self-service Star boundary for one Published Question. */
export interface QuestionStarClient {
  readonly getQuestionStar: (questionId: PublishedQuestionId) => Promise<QuestionStarProjection>;
  readonly setQuestionStar: (
    questionId: PublishedQuestionId,
    starred: boolean,
  ) => Promise<QuestionStarProjection>;
}
