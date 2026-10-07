// Closed browser command for forking one reusable Published Question Pool.

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { CreatedQuestionPool } from "./question_pool_creation";

/** The one browser command for forking an ordinary current-state Question Pool. */
export interface QuestionPoolForkClient {
  readonly forkQuestionPool: (questionPoolId: QuestionPoolId) => Promise<CreatedQuestionPool>;
}
