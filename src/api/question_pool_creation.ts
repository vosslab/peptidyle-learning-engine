// Closed browser command for creating one reusable Published Question Pool.

import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

/** Browser-supplied content for a new current-state Published Question Pool. */
export interface CreateQuestionPoolInput {
  readonly title: string;
  readonly description: string;
  readonly members: ReadonlyArray<QuestionRevisionReference>;
  readonly interchangeabilityAttested: true;
}

/** Server-issued Pool ID; a new Pool starts at Edit Number 1. */
export interface CreatedQuestionPool {
  readonly questionPoolId: QuestionId;
  readonly questionPoolEditNumber: 1;
}

/** The one browser command for creating a reusable Published Question Pool. */
export interface QuestionPoolCreationClient {
  readonly createQuestionPool: (input: CreateQuestionPoolInput) => Promise<CreatedQuestionPool>;
}
