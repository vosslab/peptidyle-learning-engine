// Browser contract for stable Published Question lineage availability.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
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
  readonly availabilityEtag: QuestionAvailabilityEtag;
};

/** Instructor Question lineage administration and exact immutable revision reads. */
export interface QuestionAvailabilityClient {
  readonly getQuestionLineage: (questionId: QuestionId) => Promise<LoadedQuestionLineage>;
  readonly getQuestionRevision: (reference: QuestionRevisionReference) => Promise<QuestionDetails>;
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
