// Strict same-origin transport for one reusable published Question Pool detail.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { QuestionPoolDetailClient } from "../question_pool_detail";
import { decodeQuestionPoolView } from "../decoders/question_pool_detail";
import { ApiProtocolError, ApiRequestError } from "./error";
import {
  browserFetch,
  encodedId,
  normalizeBasePath,
  requestSameOrigin,
  type ApiFetch,
  type HttpApiClientConfig,
} from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";

function canonicalQuestionPoolId(value: QuestionPoolId): QuestionPoolId {
  const canonical = validateCanonicalQuestionIdSyntax(value);
  if (canonical === null || canonical !== value) {
    throw new ApiProtocolError("Question Pool ID must be canonical");
  }
  return canonical;
}

async function readJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decoder(await boundedResponseJson(response, path), "response");
}

/** Creates the current Pool-detail client for the shared Library object route. */
export function createQuestionPoolDetailClient(
  config: HttpApiClientConfig = {},
): QuestionPoolDetailClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);
  return {
    getQuestionPool: async (questionPoolId): Promise<QuestionPoolView> => {
      const canonical = canonicalQuestionPoolId(questionPoolId);
      const path = `/api/question-pools/${encodedId(canonical)}`;
      const detail = await readJson(fetchImplementation, basePath, path, decodeQuestionPoolView);
      if (detail.questionPoolId !== canonical) {
        throw new ApiProtocolError("Question Pool detail does not match its requested Pool ID");
      }
      return detail;
    },
  };
}
