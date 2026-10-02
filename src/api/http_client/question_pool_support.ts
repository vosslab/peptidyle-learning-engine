// Same-origin transport for one Question Pool's optional support texts.

import {
  decodeQuestionPoolSupport,
  supportPath,
  validateQuestionPoolSupportSave,
} from "../decoders/question_pool_support";
import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { QuestionPoolSupportClient } from "../question_pool_support";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_RESPONSE_CHARACTERS = 64 * 1024;

async function responseJson(response: Response, path: string): Promise<unknown> {
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 4.1.1: the shared response helper verifies JSON Content-Type before parsing.
  return boundedResponseJson(response, path, MAX_RESPONSE_CHARACTERS);
}

function samePool(expected: QuestionPoolId, actual: QuestionPoolId, path: string): void {
  if (actual !== expected) {
    throw new ApiProtocolError(`API response ${path} returned a different Question Pool`);
  }
}

/** Composes the closed Pool support capability into the ordinary browser client. */
export function createQuestionPoolSupportClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionPoolSupportClient {
  return {
    readQuestionPoolSupport: async (
      questionPoolId,
    ): Promise<Awaited<ReturnType<QuestionPoolSupportClient["readQuestionPoolSupport"]>>> => {
      const path = supportPath(questionPoolId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      const support = decodeQuestionPoolSupport(await responseJson(response, path));
      samePool(questionPoolId, support.questionPoolId, path);
      return support;
    },
    saveQuestionPoolSupport: async (
      input,
    ): Promise<Awaited<ReturnType<QuestionPoolSupportClient["saveQuestionPoolSupport"]>>> => {
      // ASVS 2.2.1: validate the closed command before sending.
      const request = validateQuestionPoolSupportSave(input);
      const path = supportPath(request.questionPoolId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        body: {
          questionPoolMetadataEditNumber: request.questionPoolMetadataEditNumber,
          hint: request.hint,
          generalFeedback: request.generalFeedback,
          workedSolution: request.workedSolution,
        },
      });
      const support = decodeQuestionPoolSupport(await responseJson(response, path));
      samePool(request.questionPoolId, support.questionPoolId, path);
      return support;
    },
  };
}
