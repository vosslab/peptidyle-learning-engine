// Strict same-origin browser transport for private Question Collections and saved searches.

import type { QuestionCollectionReference } from "../../../generated/api/QuestionCollectionReference";
import type { QuestionCollectionSummaryView } from "../../../generated/api/QuestionCollectionSummaryView";
import type { SavedQuestionSearchReference } from "../../../generated/api/SavedQuestionSearchReference";
import type { SavedProblemSearchView } from "../../../generated/api/SavedProblemSearchView";
import type { ApiClient } from "../client";
import {
  type CreateQuestionCollectionRequest,
  type QuestionCollectionMembersPage,
  type QuestionCollectionReplaceRequest,
  type ProblemCurationClient,
  type ProblemCurationEtag,
  type RevisionedQuestionCollection,
  type RevisionedSavedProblemSearch,
  type SavedProblemSearchReplaceRequest,
} from "../problem_curation";
import {
  decodeQuestionCollectionMemberPage,
  decodeQuestionCollectionPage,
  decodeQuestionCollectionReference,
  decodeQuestionCollectionSummaryView,
  decodeQuestionCollectionQuestionIds,
  decodeProblemCurationTitle,
  decodeSavedProblemSearchFilter,
  decodeSavedProblemSearchReference,
  decodeSavedProblemSearchPage,
  decodeSavedProblemSearchView,
} from "../decoders/problem_curation";
import type { CursorPage } from "../contracts";
import { ApiProtocolError, ApiRequestError, ProblemCurationConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError("curation page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", cursor);
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

function collectionPath(collection: QuestionCollectionReference): string {
  const reference = decodeQuestionCollectionReference(collection, "collection");
  return `/api/question-collections/${encodeURIComponent(reference)}`;
}

function savedSearchPath(search: SavedQuestionSearchReference): string {
  const reference = decodeSavedProblemSearchReference(search, "search");
  return `/api/saved-problem-searches/${encodeURIComponent(reference)}`;
}

function parseStrongEtag(value: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong positive numeric validator`);
  }
  return value;
}

function requireMatchingEtag(
  response: Response,
  editNumber: string,
  path: string,
): ProblemCurationEtag {
  const etag = response.headers.get("etag");
  if (etag === null || parseStrongEtag(etag, path) !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its edit number`);
  }
  return etag;
}

function requestRevision(value: ProblemCurationEtag, path: string): string {
  return parseStrongEtag(value, `${path} If-Match`);
}

function collectionRequest(
  request: QuestionCollectionReplaceRequest,
  _path: string,
): { title?: string; questionIds: Array<string> } {
  const decoded: {
    title?: string;
    questionIds: Array<string>;
  } = {
    questionIds: decodeQuestionCollectionQuestionIds(request.questionIds, "request.questionIds"),
  };
  if (request.title !== undefined) decoded.title = decodeProblemCurationTitle(request.title);
  return decoded;
}

function createCollectionRequest(request: CreateQuestionCollectionRequest): {
  title: string;
  questionIds: Array<string>;
} {
  const decoded = collectionRequest(request, "/api/question-collections");
  if (decoded.title === undefined) {
    throw new ApiProtocolError("new named collections require a title");
  }
  return { title: decoded.title, questionIds: decoded.questionIds };
}

function savedSearchRequest(request: SavedProblemSearchReplaceRequest): {
  title: string;
  filter: import("../../../generated/api/CatalogSearchFilter").CatalogSearchFilter;
} {
  return {
    title: decodeProblemCurationTitle(request.title),
    filter: decodeSavedProblemSearchFilter(request.filter, "request.filter"),
  };
}

async function curationJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: ProblemCurationEtag;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) headers["if-match"] = requestRevision(options.etag, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ProblemCurationConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  }
  return { body: decoder(await boundedResponseJson(response, path), "response"), response };
}

async function deleteCuration(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  etag: ProblemCurationEtag,
): Promise<void> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "DELETE",
    headers: { accept: "application/json", "if-match": requestRevision(etag, path) },
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ProblemCurationConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204 || (await response.text()).length !== 0) {
    throw new ApiProtocolError(`API response ${path} must use an empty 204 response`);
  }
}

function revisionedCollection(
  result: { readonly body: QuestionCollectionSummaryView; readonly response: Response },
  path: string,
): RevisionedQuestionCollection {
  return {
    collection: result.body,
    etag: requireMatchingEtag(result.response, result.body.editNumber, path),
  };
}

function revisionedSavedSearch(
  result: { readonly body: SavedProblemSearchView; readonly response: Response },
  path: string,
): RevisionedSavedProblemSearch {
  return {
    search: result.body,
    etag: requireMatchingEtag(result.response, result.body.editNumber, path),
  };
}

/** Creates the complete curation capability without coupling it to any screen model. */
export function createProblemCurationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof ProblemCurationClient> {
  return {
    listQuestionCollections: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<QuestionCollectionSummaryView>> => {
      const path = pagePath("/api/question-collections", cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionCollectionPage,
      );
      return result.body;
    },
    getQuestionCollection: async (collection): Promise<RevisionedQuestionCollection> => {
      const path = collectionPath(collection);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionCollectionSummaryView,
      );
      return revisionedCollection(result, path);
    },
    listQuestionCollectionMembers: async (
      collection,
      cursor,
      pageSize,
    ): Promise<QuestionCollectionMembersPage> => {
      const path = pagePath(`${collectionPath(collection)}/members`, cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionCollectionMemberPage,
      );
      const etag = result.response.headers.get("etag");
      if (etag === null)
        throw new ApiProtocolError(
          `API response ${path} must include the collection edit-number ETag`,
        );
      return { page: result.body, etag: parseStrongEtag(etag, path) };
    },
    createQuestionCollection: async (request): Promise<RevisionedQuestionCollection> => {
      const path = "/api/question-collections";
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionCollectionSummaryView,
        {
          method: "POST",
          body: createCollectionRequest(request),
          expectedStatus: 201,
        },
      );
      return revisionedCollection(result, path);
    },
    replaceQuestionCollection: async (
      collection,
      request,
      etag,
    ): Promise<RevisionedQuestionCollection> => {
      const path = collectionPath(collection);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionCollectionSummaryView,
        {
          method: "PUT",
          body: collectionRequest(request, path),
          etag,
          expectedStatus: 200,
        },
      );
      return revisionedCollection(result, path);
    },
    deleteQuestionCollection: (collection, etag): Promise<void> =>
      deleteCuration(fetchImplementation, basePath, collectionPath(collection), etag),
    listSavedProblemSearches: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<SavedProblemSearchView>> => {
      const path = pagePath("/api/saved-problem-searches", cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedProblemSearchPage,
      );
      return result.body;
    },
    getSavedProblemSearch: async (search): Promise<RevisionedSavedProblemSearch> => {
      const path = savedSearchPath(search);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedProblemSearchView,
      );
      return revisionedSavedSearch(result, path);
    },
    createSavedProblemSearch: async (request): Promise<RevisionedSavedProblemSearch> => {
      const path = "/api/saved-problem-searches";
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedProblemSearchView,
        {
          method: "POST",
          body: savedSearchRequest(request),
          expectedStatus: 201,
        },
      );
      return revisionedSavedSearch(result, path);
    },
    replaceSavedProblemSearch: async (
      search,
      request,
      etag,
    ): Promise<RevisionedSavedProblemSearch> => {
      const path = savedSearchPath(search);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedProblemSearchView,
        {
          method: "PUT",
          body: savedSearchRequest(request),
          etag,
          expectedStatus: 200,
        },
      );
      return revisionedSavedSearch(result, path);
    },
    deleteSavedProblemSearch: (search, etag): Promise<void> =>
      deleteCuration(fetchImplementation, basePath, savedSearchPath(search), etag),
  };
}
