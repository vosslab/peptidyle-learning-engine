// Browser capability contract for one reusable published Question Pool detail.

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";

/** Reads one authorized current Pool detail from the shared Library route. */
export interface QuestionPoolDetailClient {
  readonly getQuestionPool: (questionPoolId: QuestionPoolId) => Promise<QuestionPoolView>;
}
