// Strict same-origin transport for reusable published Question Pool reads.

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionPoolRevisionView } from "../../../generated/api/QuestionPoolRevisionView";
import { normalizeQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";
import {
  decodeQuestionPoolLibraryPage,
  decodeQuestionPoolRevisionView,
} from "../decoders/question_pool_library";
import type { QuestionPoolLibraryClient, QuestionPoolLibraryPage } from "../question_pool_library";
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

const DEFAULT_PAGE_SIZE = 50;
const MAX_PAGE_SIZE = 100;

function canonicalQuestionPoolId(value: QuestionId): QuestionId {
  const canonical = normalizeQuestionIdSyntax(value);
  if (canonical === null || canonical !== value) {
    throw new ApiProtocolError("Question Pool ID must be canonical");
  }
  return canonical;
}

function questionPoolPagePath(cursor: string | undefined, pageSize: number): string {
  if (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE) {
    throw new ApiProtocolError("Question Pool page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams({ page_size: String(pageSize) });
  if (cursor !== undefined) query.set("cursor", cursor);
  return `/api/question-pools?${query.toString()}`;
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

/** Creates the feature-owned Pool Library client without widening the shared ApiClient. */
export function createQuestionPoolLibraryClient(
  config: HttpApiClientConfig = {},
): QuestionPoolLibraryClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);
  return {
    listQuestionPools: async (
      cursor,
      pageSize = DEFAULT_PAGE_SIZE,
    ): Promise<QuestionPoolLibraryPage> => {
      const page = await readJson(
        fetchImplementation,
        basePath,
        questionPoolPagePath(cursor, pageSize),
        decodeQuestionPoolLibraryPage,
      );
      if (page.items.length > pageSize) {
        throw new ApiProtocolError("Question Pool page exceeds its requested page size");
      }
      return page;
    },
    getQuestionPool: async (questionPoolId): Promise<QuestionPoolRevisionView> => {
      const canonical = canonicalQuestionPoolId(questionPoolId);
      const path = `/api/question-pools/${encodedId(canonical)}`;
      const detail = await readJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionPoolRevisionView,
      );
      if (detail.questionPoolRevision.questionPoolId !== canonical) {
        throw new ApiProtocolError("Question Pool detail does not match its requested Pool ID");
      }
      return detail;
    },
  };
}
