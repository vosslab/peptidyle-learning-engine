// Strict same-origin transport for one ordinary Question Pool fork.

import type { ApiClient } from "../client";
import type { CreatedQuestionPool } from "../question_pool_creation";
import type { QuestionPoolForkClient } from "../question_pool_fork";
import { decodeQuestionId } from "../decoders/shared";
import { decodeCreatedQuestionPool } from "../decoders/question_pool_creation";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

/** Composes a source-pinned server fork without accepting browser-selected content or metadata. */
export function createQuestionPoolForkClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionPoolForkClient> {
  return {
    forkQuestionPool: async (questionPoolId): Promise<CreatedQuestionPool> => {
      const canonical = decodeQuestionId(questionPoolId, "request.questionPoolId");
      const path = `/api/question-pools/${encodedId(canonical)}/fork`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
      });
      requireNoStore(response, path);
      if (response.status !== 201) throw new ApiRequestError(response.status, path);
      return decodeCreatedQuestionPool(await boundedResponseJson(response, path));
    },
  };
}
