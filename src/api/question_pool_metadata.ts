// Ordinary current metadata editing for one Question Pool.

import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";

export interface QuestionPoolMetadataReplacement {
  readonly title: string;
  readonly description: string;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
  readonly tags: ReadonlyArray<string>;
  readonly bloomCognitiveProcess: BloomCognitiveProcess | null;
  readonly bloomKnowledgeDimension: BloomKnowledgeDimension | null;
}

export interface CurrentQuestionPoolMetadata extends QuestionPoolMetadataReplacement {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
}

export interface SaveQuestionPoolMetadataRequest {
  readonly questionPoolId: QuestionPoolId;
  readonly expectedMetadataEditNumber: QuestionPoolMetadataEditNumber;
  readonly metadata: QuestionPoolMetadataReplacement;
}

export interface SavedQuestionPoolMetadata {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
}

export interface QuestionPoolMetadataClient {
  readonly getCurrentQuestionPoolMetadata: (
    questionPoolId: QuestionPoolId,
  ) => Promise<CurrentQuestionPoolMetadata>;
  readonly saveQuestionPoolMetadata: (
    request: SaveQuestionPoolMetadataRequest,
  ) => Promise<SavedQuestionPoolMetadata>;
}
