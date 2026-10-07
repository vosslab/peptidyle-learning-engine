// Complete current tuple-set replacement for one reusable Question Pool.

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolEditNumber } from "../../generated/api/QuestionPoolEditNumber";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";

export interface SaveQuestionPoolMembersInput {
  readonly questionPoolId: QuestionPoolId;
  readonly expectedQuestionPoolEditNumber: QuestionPoolEditNumber;
  readonly members: ReadonlyArray<PublishedQuestionRevisionTuple>;
}

export interface SavedQuestionPoolMembers {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
}

export interface QuestionPoolMembersClient {
  readonly saveQuestionPoolMembers: (
    input: SaveQuestionPoolMembersInput,
  ) => Promise<SavedQuestionPoolMembers>;
}
