// Strict same-origin transport for Question lineage availability and exact revisions.

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { ApiClient } from "../client";
import type {
  LoadedQuestionLineage,
  QuestionAvailabilityClient,
  QuestionAvailabilityEtag,
  QuestionAvailabilityTransition,
} from "../question_availability";
import { decodeQuestionAvailabilityTransition } from "../decoders/question_availability";
import { decodeQuestionDetails, decodeQuestionSummary } from "../decoders/question_library";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function questionPath(questionId: QuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}`;
}

function exactRevisionPath(reference: QuestionRevisionReference): string {
  return `${questionPath(reference.questionId)}/revisions/${encodeURIComponent(String(reference.revisionNumber))}`;
}

function parseStrongEtag(value: string, path: string): QuestionAvailabilityEtag {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(
      `API ${path} ETag must be one strong positive Question Availability Edit Number`,
    );
  }
  return value;
}

function responseEtag(response: Response, path: string): QuestionAvailabilityEtag {
  const value = response.headers.get("etag");
  if (value === null) {
    throw new ApiProtocolError(`API response ${path} must include a Question Availability ETag`);
  }
  return parseStrongEtag(value, path);
}

async function questionJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST";
    readonly body?: unknown;
    readonly etag?: QuestionAvailabilityEtag;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) headers["if-match"] = parseStrongEtag(options.etag, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return { body: decoder(await boundedResponseJson(response, path), "response"), response };
}

function sameQuestionRevision(
  detail: QuestionDetails,
  reference: QuestionRevisionReference,
  path: string,
): QuestionDetails {
  const actual = detail.summary.latestQuestionRevision;
  if (
    actual.questionId !== reference.questionId ||
    actual.revisionNumber !== reference.revisionNumber
  ) {
    throw new ApiProtocolError(`API response ${path} does not match its exact Question Revision`);
  }
  return detail;
}

function availabilityTransition(
  body: Omit<QuestionAvailabilityTransition, "etag">,
  response: Response,
  path: string,
): QuestionAvailabilityTransition {
  const etag = responseEtag(response, path);
  if (etag !== `"${body.editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Availability Edit Number`);
  }
  return { ...body, etag };
}

/** Composes Question availability independently from ordinary Question Library search. */
export function createQuestionAvailabilityClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionAvailabilityClient> {
  return {
    getQuestionLineage: async (questionId): Promise<LoadedQuestionLineage> => {
      const path = questionPath(questionId);
      const result = await questionJson(fetchImplementation, basePath, path, decodeQuestionSummary);
      const summary: QuestionSummary = result.body;
      if (summary.questionId !== questionId) {
        throw new ApiProtocolError(`API response ${path} does not match its Question lineage`);
      }
      return { summary, availabilityEtag: responseEtag(result.response, path) };
    },
    getQuestionRevision: async (reference): Promise<QuestionDetails> => {
      const path = exactRevisionPath(reference);
      const result = await questionJson(fetchImplementation, basePath, path, decodeQuestionDetails);
      return sameQuestionRevision(result.body, reference, path);
    },
    archiveQuestion: async (
      questionId,
      confirmationTitle,
      etag,
    ): Promise<QuestionAvailabilityTransition> => {
      const path = `${questionPath(questionId)}/archive`;
      const result = await questionJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionAvailabilityTransition,
        { method: "POST", body: { confirmationTitle }, etag },
      );
      return availabilityTransition(result.body, result.response, path);
    },
    restoreQuestion: async (questionId, etag): Promise<QuestionAvailabilityTransition> => {
      const path = `${questionPath(questionId)}/restore`;
      const result = await questionJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionAvailabilityTransition,
        { method: "POST", etag },
      );
      return availabilityTransition(result.body, result.response, path);
    },
  };
}
