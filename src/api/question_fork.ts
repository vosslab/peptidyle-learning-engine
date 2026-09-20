// Closed browser command for creating one private Draft from an exact Published Question Revision.

import type { DraftQuestionRouteId } from "../navigation/public_route";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";

/** Opaque browser retry token bound by the server to one active Instructor and source Revision. */
export type QuestionForkIdempotencyKey = string;

/** Answer-free navigation receipt for one server-created private Draft Question. */
export interface ForkedPublishedQuestion {
  readonly draftQuestion: DraftQuestionRouteId;
}

/** The one browser command for forking an exact Published Question Revision. */
export interface QuestionForkClient {
  readonly forkPublishedQuestion: (
    sourceRevisionTuple: QuestionRevisionTuple,
    requestKey: QuestionForkIdempotencyKey,
  ) => Promise<ForkedPublishedQuestion>;
}
