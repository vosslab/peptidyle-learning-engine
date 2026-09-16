// Closed browser command for creating one reusable Published Question Pool.

import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

/** Browser-supplied content for a server-issued Published Question Pool Revision 1. */
export interface CreateQuestionPoolInput {
  readonly members: ReadonlyArray<QuestionRevisionReference>;
  readonly interchangeabilityAttested: true;
}

/** Server-issued identity and first immutable revision for one Published Question Pool. */
export interface CreatedQuestionPool {
  readonly questionPoolId: QuestionId;
  readonly revisionNumber: 1;
}

/** The one browser command for creating a reusable Published Question Pool. */
export interface QuestionPoolCreationClient {
  readonly createQuestionPool: (input: CreateQuestionPoolInput) => Promise<CreatedQuestionPool>;
}
