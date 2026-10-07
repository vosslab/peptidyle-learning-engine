// Same-origin ordinary Published Question metadata save.

import type { SaveQuestionMetadataRequest } from "../../../generated/api/SaveQuestionMetadataRequest";
import type { SavedQuestionMetadata } from "../../../generated/api/SavedQuestionMetadata";
import { decodeSavedQuestionMetadata } from "../decoders/question_metadata";
import type { QuestionMetadataClient } from "../question_metadata";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function questionMetadataPath(request: SaveQuestionMetadataRequest): string {
  const tuple = request.publishedQuestionRevisionTuple;
  if (
    !Number.isSafeInteger(tuple.revisionNumber) ||
    tuple.revisionNumber < 1 ||
    tuple.revisionNumber > 4_294_967_295 ||
    !Number.isSafeInteger(request.expectedMetadataEditNumber) ||
    request.expectedMetadataEditNumber < 1
  ) {
    throw new ApiProtocolError("Question metadata save requires exact positive concurrency values");
  }
  return `/api/questions/by-id/${encodedId(tuple.publishedQuestionId)}/metadata`;
}

/** Composes the exact-tuple ordinary metadata save capability. */
export function createQuestionMetadataClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionMetadataClient {
  return {
    saveQuestionMetadata: async (request): Promise<SavedQuestionMetadata> => {
      const path = questionMetadataPath(request);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        body: request,
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const saved = decodeSavedQuestionMetadata(await boundedResponseJson(response, path));
      const expected = request.publishedQuestionRevisionTuple;
      const actual = saved.publishedQuestionRevisionTuple;
      if (
        actual.publishedQuestionId !== expected.publishedQuestionId ||
        actual.revisionNumber !== expected.revisionNumber
      ) {
        throw new ApiProtocolError(
          `API response ${path} does not match its exact Question Revision`,
        );
      }
      return saved;
    },
  };
}
