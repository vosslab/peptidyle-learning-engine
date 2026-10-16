// Strict same-origin transport for the Published Question Star projection.

import type { PublishedQuestionId } from "../../../generated/api/PublishedQuestionId";
import type { LibraryObjectSearchRequest } from "../../../generated/api/LibraryObjectSearchRequest";
import type { LibraryFacetField } from "../../../generated/api/LibraryFacetField";
import type { LibraryFacetChoicePage } from "../../../generated/api/LibraryFacetChoicePage";
import type { ApiClient } from "../client";
import { decodeQuestionStarProjection, decodeStarredQuestionPage } from "../decoders/question_star";
import type {
  QuestionStarClient,
  QuestionStarProjection,
  StarredQuestionPage,
  StarredQuestionSort,
} from "../question_star";
import { libraryObjectSearchPath } from "../library_object_search_query";
import { decodeLibraryFacetChoicePage } from "../decoders/question_library";
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
  query: LibraryObjectSearchRequest,
  sort: StarredQuestionSort,
): Promise<StarredQuestionPage> {
  if (
    query.cursor !== null &&
    (query.cursor.length === 0 || query.cursor.length > MAX_COLLECTION_CURSOR_LENGTH)
  ) {
    throw new Error("Starred Question cursor is invalid");
  }
  const publicPath = libraryObjectSearchPath(query);
  const parameters = new URLSearchParams(publicPath.split("?", 2)[1]);
  parameters.set("starredSort", sort);
  const path = `${STARRED_QUESTIONS_PATH}?${parameters.toString()}`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "GET",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeStarredQuestionPage(await boundedResponseJson(response, path));
}

async function requestStarredFacetChoices(
  fetchImplementation: ApiFetch,
  basePath: string,
  field: LibraryFacetField,
  query: LibraryObjectSearchRequest,
  sort: StarredQuestionSort,
  search: string,
): Promise<LibraryFacetChoicePage> {
  const publicPath = libraryObjectSearchPath(query);
  const parameters = new URLSearchParams(publicPath.split("?", 2)[1]);
  parameters.set("starredSort", sort);
  if (search.length > 0) parameters.set("choice_search", search);
  const path = `${STARRED_QUESTIONS_PATH}/facet-choices/${encodeURIComponent(field)}?${parameters.toString()}`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method: "GET" });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeLibraryFacetChoicePage(await boundedResponseJson(response, path));
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
    listStarredQuestions: (query, sort) =>
      requestStarredQuestions(fetchImplementation, basePath, query, sort),
    listStarredFacetChoices: (field, query, sort, search) =>
      requestStarredFacetChoices(fetchImplementation, basePath, field, query, sort, search),
  };
}
