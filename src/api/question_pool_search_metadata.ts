// Closed browser capability for Question Pool Topic, Subtopic, and Tags.

import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";
import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { Tag } from "../../generated/api/Tag";

/** Discipline and Subject are established by the first member and are not patch fields. */
export interface QuestionPoolSearchMetadataPatch {
  readonly tags?: ReadonlyArray<Tag>;
  readonly topicUuid?: string | null;
  readonly subtopicUuid?: string | null;
}

export interface QuestionPoolSearchMetadataSelectionItem {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
}

export interface QuestionPoolSearchMetadataUpdateRequest {
  readonly selection: ReadonlyArray<QuestionPoolSearchMetadataSelectionItem>;
  readonly patch: QuestionPoolSearchMetadataPatch;
}

export interface QuestionPoolSearchMetadataUpdateResult {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
}

/** The only browser operation for replacing Pool-owned search metadata. */
export interface QuestionPoolSearchMetadataClient {
  readonly updateQuestionPoolSearchMetadata: (
    request: QuestionPoolSearchMetadataUpdateRequest,
  ) => Promise<ReadonlyArray<QuestionPoolSearchMetadataUpdateResult>>;
}
