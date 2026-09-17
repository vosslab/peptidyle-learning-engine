// Closed browser command for creating one private Draft from an exact Published Question Revision.

import type { DraftQuestionReference } from "../../generated/api/DraftQuestionReference";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

/** Opaque browser retry token bound by the server to one active Instructor and source Revision. */
export type QuestionForkIdempotencyKey = string;

/** Answer-free navigation receipt for one server-created private Draft Question. */
export interface ForkedPublishedQuestion {
  readonly draftQuestion: DraftQuestionReference;
}

/** The one browser command for forking an exact Published Question Revision. */
export interface QuestionForkClient {
  readonly forkPublishedQuestion: (
    source: QuestionRevisionReference,
    requestKey: QuestionForkIdempotencyKey,
  ) => Promise<ForkedPublishedQuestion>;
}
