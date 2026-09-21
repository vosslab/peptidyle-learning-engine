// Same-origin explicit own-state transport for one published Pool.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import {
  decodeQuestionPoolStarProjection,
  decodeQuestionPoolWatchProjection,
} from "../decoders/question_pool_stewardship";
import type {
  QuestionPoolStewardshipClient,
  QuestionPoolStarProjection,
  QuestionPoolWatchProjection,
} from "../question_pool_stewardship";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

async function requestStar(
  fetchImplementation: ApiFetch,
  basePath: string,
  poolId: QuestionPoolId,
  starred?: boolean,
): Promise<QuestionPoolStarProjection> {
  const path = `/api/question-pools/${encodedId(poolId)}/stewardship/star`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: starred === undefined ? "GET" : "PUT",
    body: starred === undefined ? undefined : { starred },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeQuestionPoolStarProjection(await boundedResponseJson(response, path));
}

async function requestWatch(
  fetchImplementation: ApiFetch,
  basePath: string,
  poolId: QuestionPoolId,
  watching?: boolean,
): Promise<QuestionPoolWatchProjection> {
  const path = `/api/question-pools/${encodedId(poolId)}/stewardship/watch`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: watching === undefined ? "GET" : "PUT",
    body: watching === undefined ? undefined : { watching },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeQuestionPoolWatchProjection(await boundedResponseJson(response, path));
}

/** Composes four self-service methods without collection or recipient APIs. */
export function createQuestionPoolStewardshipClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionPoolStewardshipClient {
  return {
    getQuestionPoolStar: (poolId) => requestStar(fetchImplementation, basePath, poolId),
    setQuestionPoolStar: (poolId, starred) =>
      requestStar(fetchImplementation, basePath, poolId, starred),
    getQuestionPoolWatch: (poolId) => requestWatch(fetchImplementation, basePath, poolId),
    setQuestionPoolWatch: (poolId, watching) =>
      requestWatch(fetchImplementation, basePath, poolId, watching),
  };
}
