// Strict same-origin transport for self-only Published Question Watch state.

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { ApiClient } from "../client";
import { decodeQuestionWatchProjection } from "../decoders/question_watch";
import type { QuestionWatchClient, QuestionWatchProjection } from "../question_watch";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function watchPath(questionId: QuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}/stewardship/watch`;
}

async function requestWatch(
  fetchImplementation: ApiFetch,
  basePath: string,
  questionId: QuestionId,
  watching?: boolean,
): Promise<QuestionWatchProjection> {
  const path = watchPath(questionId);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: watching === undefined ? "GET" : "PUT",
    body: watching === undefined ? undefined : { watching },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeQuestionWatchProjection(await boundedResponseJson(response, path));
}

/** Composes the closed private Watch endpoint without a collection/list API. */
export function createQuestionWatchClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionWatchClient> {
  return {
    getQuestionWatch: (questionId) => requestWatch(fetchImplementation, basePath, questionId),
    setQuestionWatch: (questionId, watching) =>
      requestWatch(fetchImplementation, basePath, questionId, watching),
  };
}
