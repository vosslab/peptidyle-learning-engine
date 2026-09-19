// Browser contract for stable Published Question lineage availability.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";

export type QuestionAvailabilityEtag = string;

export type QuestionAvailabilityTransition = {
  readonly availability: "available" | "archived";
  readonly editNumber: string;
  readonly etag: QuestionAvailabilityEtag;
};

/** Current discoverable lineage state and its qualified availability validator. */
export type LoadedQuestionLineage = {
  readonly summary: QuestionSummary;
  readonly viewerMayArchive: boolean;
  readonly availabilityEtag: QuestionAvailabilityEtag;
};

/** Instructor Question lineage administration and exact immutable revision reads. */
export interface QuestionAvailabilityClient {
  readonly getQuestionLineage: (questionId: QuestionId) => Promise<LoadedQuestionLineage>;
  readonly getQuestionRevision: (questionRevisionTuple: QuestionRevisionTuple) => Promise<QuestionDetails>;
  /** Same-origin, answer-free WeBWorK preview for one exact immutable Revision. */
  readonly questionRevisionPreviewDocumentUrl: (questionRevisionTuple: QuestionRevisionTuple) => string;
  readonly archiveQuestion: (
    questionId: QuestionId,
    confirmationTitle: string,
    etag: QuestionAvailabilityEtag,
  ) => Promise<QuestionAvailabilityTransition>;
  readonly restoreQuestion: (
    questionId: QuestionId,
    etag: QuestionAvailabilityEtag,
  ) => Promise<QuestionAvailabilityTransition>;
}
