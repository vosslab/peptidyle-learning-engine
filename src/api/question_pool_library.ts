// Browser capability contract for reusable published Question Pool reads.

import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionPoolLibrarySummary } from "../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolRevisionView } from "../../generated/api/QuestionPoolRevisionView";

export interface QuestionPoolLibraryPage {
  readonly items: ReadonlyArray<QuestionPoolLibrarySummary>;
  readonly nextCursor: string | null;
}

/** Dedicated read boundary for selecting an existing published Question Pool. */
export interface QuestionPoolLibraryClient {
  readonly listQuestionPools: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<QuestionPoolLibraryPage>;
  readonly getQuestionPool: (questionPoolId: QuestionId) => Promise<QuestionPoolRevisionView>;
}
