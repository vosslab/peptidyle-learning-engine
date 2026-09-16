// Closed browser capability for current shared Published Question metadata.

import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { Tag } from "../../generated/api/Tag";

export interface QuestionBulkMetadataPatch {
  readonly tags?: ReadonlyArray<Tag>;
  readonly disciplineUuid?: string;
  readonly subjectUuid?: string;
  readonly topicUuid?: string | null;
  readonly subtopicUuid?: string | null;
}

export interface QuestionBulkMetadataSelectionItem {
  readonly questionId: QuestionId;
  readonly metadataEditNumber: number;
}

export interface QuestionBulkMetadataUpdateRequest {
  readonly selection: ReadonlyArray<QuestionBulkMetadataSelectionItem>;
  readonly patch: QuestionBulkMetadataPatch;
}

export interface QuestionBulkMetadataUpdateResult {
  readonly questionId: QuestionId;
  readonly metadataEditNumber: number;
}

/** The only browser operations for reading and replacing shared search metadata. */
export interface QuestionBulkMetadataClient {
  readonly getCurrentQuestionBulkMetadata: (
    questionIds: ReadonlyArray<QuestionId>,
  ) => Promise<ReadonlyArray<PublishedQuestionSharedMetadata>>;
  readonly updateQuestionBulkMetadata: (
    request: QuestionBulkMetadataUpdateRequest,
  ) => Promise<ReadonlyArray<QuestionBulkMetadataUpdateResult>>;
}
