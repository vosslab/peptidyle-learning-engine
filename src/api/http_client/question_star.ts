// Strict same-origin transport for the Published Question Star projection.

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { ApiClient } from "../client";
import { decodeQuestionStarProjection } from "../decoders/question_star";
import type { QuestionStarClient, QuestionStarProjection } from "../question_star";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function starPath(questionId: QuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}/stewardship/star`;
}

async function requestStar(
  fetchImplementation: ApiFetch,
  basePath: string,
  questionId: QuestionId,
  starred?: boolean,
): Promise<QuestionStarProjection> {
  const path = starPath(questionId);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: starred === undefined ? "GET" : "PUT",
    body: starred === undefined ? undefined : { starred },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeQuestionStarProjection(await boundedResponseJson(response, path));
}

/** Composes the one closed Star endpoint; it cannot look up another profile. */
export function createQuestionStarClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionStarClient> {
  return {
    getQuestionStar: (questionId) => requestStar(fetchImplementation, basePath, questionId),
    setQuestionStar: (questionId, starred) =>
      requestStar(fetchImplementation, basePath, questionId, starred),
  };
}
