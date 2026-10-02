// Strict same-origin transport for the Published Question Star projection.

import type { PublishedQuestionId } from "../../../generated/api/PublishedQuestionId";
import type { ApiClient } from "../client";
import { decodeQuestionStarProjection, decodeStarredQuestionPage } from "../decoders/question_star";
import type {
  QuestionStarClient,
  QuestionStarProjection,
  StarredQuestionPage,
} from "../question_star";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function starPath(questionId: PublishedQuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}/stewardship/star`;
}

async function requestStar(
  fetchImplementation: ApiFetch,
  basePath: string,
  questionId: PublishedQuestionId,
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

const STARRED_QUESTIONS_PATH = "/api/questions/stewardship/stars";
const MAX_COLLECTION_CURSOR_LENGTH = 1024;

async function requestStarredQuestions(
  fetchImplementation: ApiFetch,
  basePath: string,
  cursor: string | null,
  pageSize: 50 | 100 | 250,
): Promise<StarredQuestionPage> {
  if (cursor !== null && (cursor.length === 0 || cursor.length > MAX_COLLECTION_CURSOR_LENGTH)) {
    throw new Error("Starred Question cursor is invalid");
  }
  const parameters = new URLSearchParams({ pageSize: String(pageSize) });
  if (cursor !== null) parameters.set("cursor", cursor);
  const path = `${STARRED_QUESTIONS_PATH}?${parameters.toString()}`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "GET",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeStarredQuestionPage(await boundedResponseJson(response, path));
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
    listStarredQuestions: (cursor, pageSize) =>
      requestStarredQuestions(fetchImplementation, basePath, cursor, pageSize),
  };
}
