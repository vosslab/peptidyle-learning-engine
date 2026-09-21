// Browser contract for stable Published Question lineage availability.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";
import type { QuestionAvailabilityEditNumber } from "../../generated/api/QuestionAvailabilityEditNumber";

export type QuestionAvailabilityTransition = {
  readonly availability: "available" | "archived";
  readonly questionAvailabilityEditNumber: QuestionAvailabilityEditNumber;
};

/** Current discoverable lineage state and its qualified availability validator. */
export type LoadedQuestionLineage = {
  readonly summary: QuestionSummary;
  readonly viewerMayArchive: boolean;
  readonly questionAvailabilityEditNumber: QuestionAvailabilityEditNumber;
};

/** Instructor Question lineage administration and exact immutable revision reads. */
export interface QuestionAvailabilityClient {
  readonly getQuestionLineage: (questionId: PublishedQuestionId) => Promise<LoadedQuestionLineage>;
  readonly getQuestionRevision: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => Promise<QuestionDetails>;
  /** Same-origin, answer-free WeBWorK preview for one exact immutable Revision. */
  readonly questionRevisionPreviewDocumentUrl: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly archiveQuestion: (
    questionId: PublishedQuestionId,
    confirmationTitle: string,
    expectedQuestionAvailabilityEditNumber: QuestionAvailabilityEditNumber,
  ) => Promise<QuestionAvailabilityTransition>;
  readonly restoreQuestion: (
    questionId: PublishedQuestionId,
    expectedQuestionAvailabilityEditNumber: QuestionAvailabilityEditNumber,
  ) => Promise<QuestionAvailabilityTransition>;
}
