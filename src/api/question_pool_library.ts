// Browser capability contract for reusable published Question Pool reads.

import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionPoolLibrarySummary } from "../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolBloomFacets } from "../../generated/api/QuestionPoolBloomFacets";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import type { LibraryClassificationFilter } from "./library_classification_filter";

export interface QuestionPoolLibraryFilter extends LibraryClassificationFilter {
  readonly text?: string | null;
  readonly tags?: ReadonlyArray<string>;
  readonly bloom_cognitive_process?: BloomCognitiveProcess | null;
  readonly bloom_knowledge_dimension?: BloomKnowledgeDimension | null;
}

export interface QuestionPoolLibraryPage {
  readonly items: ReadonlyArray<QuestionPoolLibrarySummary>;
  readonly nextCursor: string | null;
  readonly bloomFacets: QuestionPoolBloomFacets;
}

/** Dedicated read boundary for selecting an existing published Question Pool. */
export interface QuestionPoolLibraryClient {
  readonly listQuestionPools: (
    cursor?: string,
    pageSize?: number,
    filter?: QuestionPoolLibraryFilter,
  ) => Promise<QuestionPoolLibraryPage>;
  readonly getQuestionPool: (questionPoolId: QuestionId) => Promise<QuestionPoolView>;
}
