// Strict same-origin browser transport for private Question Curation records.

import type { QuestionFolderReference } from "../../../generated/api/QuestionFolderReference";
import type { QuestionFolderSummaryView } from "../../../generated/api/QuestionFolderSummaryView";
import type { SavedQuestionSearchReference } from "../../../generated/api/SavedQuestionSearchReference";
import type { SavedQuestionSearchView } from "../../../generated/api/SavedQuestionSearchView";
import type { ApiClient } from "../client";
import {
  type CreateQuestionFolderRequest,
  type QuestionFolderEntriesPage,
  type QuestionFolderReplaceRequest,
  type QuestionCurationClient,
  type QuestionCurationEtag,
  type RevisionedQuestionFolder,
  type RevisionedSavedQuestionSearch,
  type SavedQuestionSearchReplaceRequest,
} from "../question_curation";
import {
  decodeQuestionFolderEntryPage,
  decodeQuestionFolderPage,
  decodeQuestionFolderReference,
  decodeQuestionFolderSummaryView,
  decodeQuestionFolderQuestionIds,
  decodeQuestionCurationTitle,
  decodeSavedQuestionSearchFilter,
  decodeSavedQuestionSearchReference,
  decodeSavedQuestionSearchPage,
  decodeSavedQuestionSearchView,
} from "../decoders/question_curation";
import type { CursorPage } from "../contracts";
import { ApiProtocolError, ApiRequestError, QuestionCurationConflictError } from "./error";
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

function folderPath(folder: QuestionFolderReference): string {
  const reference = decodeQuestionFolderReference(folder, "folder");
  return `/api/question-folders/${encodeURIComponent(reference)}`;
}

function savedSearchPath(search: SavedQuestionSearchReference): string {
  const reference = decodeSavedQuestionSearchReference(search, "search");
  return `/api/saved-question-searches/${encodeURIComponent(reference)}`;
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
): QuestionCurationEtag {
  const etag = response.headers.get("etag");
  if (etag === null || parseStrongEtag(etag, path) !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its edit number`);
  }
  return etag;
}

function requestRevision(value: QuestionCurationEtag, path: string): string {
  return parseStrongEtag(value, `${path} If-Match`);
}

function folderRequest(
  request: QuestionFolderReplaceRequest,
  _path: string,
): { title?: string; questionIds: Array<string> } {
  const decoded: {
    title?: string;
    questionIds: Array<string>;
  } = {
    questionIds: decodeQuestionFolderQuestionIds(request.questionIds, "request.questionIds"),
  };
  if (request.title !== undefined) decoded.title = decodeQuestionCurationTitle(request.title);
  return decoded;
}

function createFolderRequest(request: CreateQuestionFolderRequest): {
  title: string;
  questionIds: Array<string>;
} {
  const decoded = folderRequest(request, "/api/question-folders");
  if (decoded.title === undefined) {
    throw new ApiProtocolError("new named Question Folders require a title");
  }
  return { title: decoded.title, questionIds: decoded.questionIds };
}

function savedSearchRequest(request: SavedQuestionSearchReplaceRequest): {
  title: string;
  filter: import("../../../generated/api/QuestionSearchFilter").QuestionSearchFilter;
} {
  return {
    title: decodeQuestionCurationTitle(request.title),
    filter: decodeSavedQuestionSearchFilter(request.filter, "request.filter"),
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
    readonly etag?: QuestionCurationEtag;
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
  if (response.status === 412) throw new QuestionCurationConflictError(path);
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
  etag: QuestionCurationEtag,
): Promise<void> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "DELETE",
    headers: { accept: "application/json", "if-match": requestRevision(etag, path) },
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new QuestionCurationConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204 || (await response.text()).length !== 0) {
    throw new ApiProtocolError(`API response ${path} must use an empty 204 response`);
  }
}

function revisionedFolder(
  result: { readonly body: QuestionFolderSummaryView; readonly response: Response },
  path: string,
): RevisionedQuestionFolder {
  return {
    folder: result.body,
    etag: requireMatchingEtag(result.response, result.body.editNumber, path),
  };
}

function revisionedSavedSearch(
  result: { readonly body: SavedQuestionSearchView; readonly response: Response },
  path: string,
): RevisionedSavedQuestionSearch {
  return {
    search: result.body,
    etag: requireMatchingEtag(result.response, result.body.editNumber, path),
  };
}

/** Creates the complete curation capability without coupling it to any screen model. */
export function createQuestionCurationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionCurationClient> {
  return {
    listQuestionFolders: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<QuestionFolderSummaryView>> => {
      const path = pagePath("/api/question-folders", cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionFolderPage,
      );
      return result.body;
    },
    getQuestionFolder: async (folder): Promise<RevisionedQuestionFolder> => {
      const path = folderPath(folder);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionFolderSummaryView,
      );
      return revisionedFolder(result, path);
    },
    listQuestionFolderEntries: async (
      folder,
      cursor,
      pageSize,
    ): Promise<QuestionFolderEntriesPage> => {
      const path = pagePath(`${folderPath(folder)}/entries`, cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionFolderEntryPage,
      );
      const etag = result.response.headers.get("etag");
      if (etag === null)
        throw new ApiProtocolError(
          `API response ${path} must include the Question Folder edit-number ETag`,
        );
      return { page: result.body, etag: parseStrongEtag(etag, path) };
    },
    createQuestionFolder: async (request): Promise<RevisionedQuestionFolder> => {
      const path = "/api/question-folders";
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionFolderSummaryView,
        {
          method: "POST",
          body: createFolderRequest(request),
          expectedStatus: 201,
        },
      );
      return revisionedFolder(result, path);
    },
    replaceQuestionFolder: async (folder, request, etag): Promise<RevisionedQuestionFolder> => {
      const path = folderPath(folder);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionFolderSummaryView,
        {
          method: "PUT",
          body: folderRequest(request, path),
          etag,
          expectedStatus: 200,
        },
      );
      return revisionedFolder(result, path);
    },
    deleteQuestionFolder: (folder, etag): Promise<void> =>
      deleteCuration(fetchImplementation, basePath, folderPath(folder), etag),
    listSavedQuestionSearches: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<SavedQuestionSearchView>> => {
      const path = pagePath("/api/saved-question-searches", cursor, pageSize);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedQuestionSearchPage,
      );
      return result.body;
    },
    getSavedQuestionSearch: async (search): Promise<RevisionedSavedQuestionSearch> => {
      const path = savedSearchPath(search);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedQuestionSearchView,
      );
      return revisionedSavedSearch(result, path);
    },
    createSavedQuestionSearch: async (request): Promise<RevisionedSavedQuestionSearch> => {
      const path = "/api/saved-question-searches";
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedQuestionSearchView,
        {
          method: "POST",
          body: savedSearchRequest(request),
          expectedStatus: 201,
        },
      );
      return revisionedSavedSearch(result, path);
    },
    replaceSavedQuestionSearch: async (
      search,
      request,
      etag,
    ): Promise<RevisionedSavedQuestionSearch> => {
      const path = savedSearchPath(search);
      const result = await curationJson(
        fetchImplementation,
        basePath,
        path,
        decodeSavedQuestionSearchView,
        {
          method: "PUT",
          body: savedSearchRequest(request),
          etag,
          expectedStatus: 200,
        },
      );
      return revisionedSavedSearch(result, path);
    },
    deleteSavedQuestionSearch: (search, etag): Promise<void> =>
      deleteCuration(fetchImplementation, basePath, savedSearchPath(search), etag),
  };
}
