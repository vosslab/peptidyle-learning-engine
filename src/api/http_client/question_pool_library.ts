// Strict same-origin transport for reusable published Question Pool reads.

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionPoolRevisionView } from "../../../generated/api/QuestionPoolRevisionView";
import type { QuestionPoolRevisionReference } from "../../../generated/api/QuestionPoolRevisionReference";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";
import {
  decodeQuestionPoolLibraryPage,
  decodeQuestionPoolRevisionView,
} from "../decoders/question_pool_library";
import type {
  QuestionPoolLibraryClient,
  QuestionPoolLibraryPage,
  QuestionPoolLibraryFilter,
} from "../question_pool_library";
import { questionPoolLibraryFilter } from "../question_pool_library_filter";
import {
  appendLibraryClassificationParameters,
  EMPTY_LIBRARY_CLASSIFICATION_FILTER,
} from "../library_classification_filter";
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
  const canonical = validateCanonicalQuestionIdSyntax(value);
  if (canonical === null || canonical !== value) {
    throw new ApiProtocolError("Question Pool ID must be canonical");
  }
  return canonical;
}

function exactQuestionPoolRevisionPath(reference: QuestionPoolRevisionReference): string {
  const canonical = canonicalQuestionPoolId(reference.questionPoolId);
  if (!Number.isSafeInteger(reference.revisionNumber) || reference.revisionNumber < 1) {
    throw new ApiProtocolError("Question Pool Revision Number must be a positive safe integer");
  }
  return `/api/question-pools/${encodedId(canonical)}/revisions/${reference.revisionNumber}`;
}

function questionPoolPagePath(
  cursor: string | undefined,
  pageSize: number,
  value: QuestionPoolLibraryFilter,
): string {
  if (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE) {
    throw new ApiProtocolError("Question Pool page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams({ page_size: String(pageSize) });
  const filter = questionPoolLibraryFilter(value);
  if (cursor !== undefined) query.set("cursor", cursor);
  // ASVS 1.2.2/2.2.1: encode only validated, allowlisted classification identities.
  appendLibraryClassificationParameters(query, filter);
  if (filter.text) query.set("text", filter.text);
  for (const tag of filter.tags ?? []) query.append("tags", tag);
  if (filter.bloom_cognitive_process !== null && filter.bloom_cognitive_process !== undefined) {
    query.set("bloom_cognitive_process", filter.bloom_cognitive_process);
  }
  if (filter.bloom_knowledge_dimension !== null && filter.bloom_knowledge_dimension !== undefined) {
    query.set("bloom_knowledge_dimension", filter.bloom_knowledge_dimension);
  }
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
      filter = EMPTY_LIBRARY_CLASSIFICATION_FILTER,
    ): Promise<QuestionPoolLibraryPage> => {
      const page = await readJson(
        fetchImplementation,
        basePath,
        questionPoolPagePath(cursor, pageSize, filter),
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
    getQuestionPoolRevision: async (reference): Promise<QuestionPoolRevisionView> => {
      const path = exactQuestionPoolRevisionPath(reference);
      const detail = await readJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionPoolRevisionView,
      );
      if (
        detail.questionPoolRevision.questionPoolId !== reference.questionPoolId ||
        detail.questionPoolRevision.revisionNumber !== reference.revisionNumber
      ) {
        throw new ApiProtocolError(
          "Question Pool detail does not match its requested exact Pool Revision",
        );
      }
      return detail;
    },
  };
}
