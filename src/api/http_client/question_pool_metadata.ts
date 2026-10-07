// Same-origin ordinary Question Pool metadata read and replacement.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type {
  CurrentQuestionPoolMetadata,
  QuestionPoolMetadataClient,
  SavedQuestionPoolMetadata,
} from "../question_pool_metadata";
import {
  decodeCurrentQuestionPoolMetadata,
  decodeSavedQuestionPoolMetadata,
  validateQuestionPoolMetadataRequest,
} from "../decoders/question_pool_metadata";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function metadataPath(questionPoolId: QuestionPoolId): string {
  return `/api/question-pools/${encodedId(questionPoolId)}/metadata`;
}

async function decodeResponse<T>(
  response: Response,
  path: string,
  decode: (value: unknown) => T,
): Promise<T> {
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decode(await boundedResponseJson(response, path));
}

function requireSamePool(expected: QuestionPoolId, actual: QuestionPoolId, path: string): void {
  if (expected !== actual) {
    throw new ApiProtocolError(`API response ${path} does not match its exact Question Pool`);
  }
}

export function createQuestionPoolMetadataClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionPoolMetadataClient {
  return {
    getCurrentQuestionPoolMetadata: async (
      questionPoolId,
    ): Promise<CurrentQuestionPoolMetadata> => {
      const path = metadataPath(questionPoolId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      const metadata = await decodeResponse(response, path, decodeCurrentQuestionPoolMetadata);
      requireSamePool(questionPoolId, metadata.questionPoolId, path);
      return metadata;
    },
    saveQuestionPoolMetadata: async (input): Promise<SavedQuestionPoolMetadata> => {
      const request = validateQuestionPoolMetadataRequest(input);
      const path = metadataPath(request.questionPoolId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        body: request,
      });
      const saved = await decodeResponse(response, path, decodeSavedQuestionPoolMetadata);
      requireSamePool(request.questionPoolId, saved.questionPoolId, path);
      return saved;
    },
  };
}
