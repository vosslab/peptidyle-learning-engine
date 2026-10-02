// Same-origin transport for one Question Pool Topic, Subtopic, and Tag command.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import {
  decodeQuestionPoolSearchMetadataResult,
  validateQuestionPoolSearchMetadataRequest,
} from "../decoders/question_pool_search_metadata";
import type { QuestionPoolSearchMetadataClient } from "../question_pool_search_metadata";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const UPDATE_PATH = "/api/question-pools/bulk-search-metadata";
const MAX_RESPONSE_CHARACTERS = 1024 * 1024;

function exactOrderedIdSet(
  expected: ReadonlyArray<QuestionPoolId>,
  actual: ReadonlyArray<{ readonly questionPoolId: QuestionPoolId }>,
  path: string,
): void {
  if (
    actual.length !== expected.length ||
    actual.some((item, index) => item.questionPoolId !== expected[index])
  ) {
    throw new ApiProtocolError(
      `API response ${path} must contain the exact requested Question Pool set`,
    );
  }
}

async function responseJson(response: Response, path: string): Promise<unknown> {
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 4.1.1: the shared response helper verifies JSON Content-Type before parsing.
  return boundedResponseJson(response, path, MAX_RESPONSE_CHARACTERS);
}

/** Composes the closed Pool search-metadata capability into the ordinary browser client. */
export function createQuestionPoolSearchMetadataClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionPoolSearchMetadataClient {
  return {
    updateQuestionPoolSearchMetadata: async (
      input,
    ): Promise<
      Awaited<ReturnType<QuestionPoolSearchMetadataClient["updateQuestionPoolSearchMetadata"]>>
    > => {
      // ASVS 2.2.1: validate the closed command before sending.
      const request = validateQuestionPoolSearchMetadataRequest(input);
      if (request.selection.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
        throw new ApiProtocolError("Question Pool search metadata selection is too large");
      }
      const response = await requestSameOrigin(fetchImplementation, basePath, UPDATE_PATH, {
        method: "POST",
        body: request,
      });
      const results = decodeQuestionPoolSearchMetadataResult(
        await responseJson(response, UPDATE_PATH),
      );
      exactOrderedIdSet(
        request.selection.map((item) => item.questionPoolId),
        results,
        UPDATE_PATH,
      );
      return results;
    },
  };
}
