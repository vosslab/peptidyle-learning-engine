// Ordinary single-Question metadata correction, bound to one exact current Revision.

import type { SaveQuestionMetadataRequest } from "../../generated/api/SaveQuestionMetadataRequest";
import type { SavedQuestionMetadata } from "../../generated/api/SavedQuestionMetadata";

export interface QuestionMetadataClient {
  readonly saveQuestionMetadata: (
    request: SaveQuestionMetadataRequest,
  ) => Promise<SavedQuestionMetadata>;
}
