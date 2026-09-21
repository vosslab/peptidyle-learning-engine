// Strict same-origin transport for Question lineage availability and exact revisions.

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../../generated/api/PublishedQuestionId";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { ApiClient } from "../client";
import type {
  LoadedQuestionLineage,
  QuestionAvailabilityClient,
  QuestionAvailabilityTransition,
} from "../question_availability";
import type { QuestionAvailabilityEditNumber } from "../../../generated/api/QuestionAvailabilityEditNumber";
import {
  assertResponseMatchesPositiveNumber,
  ifMatchHeaderForPositiveNumber,
  numberFromQuotedPositiveHeader,
} from "./conditional_request";
import {
  decodeQuestionAvailabilityTransition,
  decodeQuestionLineageView,
} from "../decoders/question_availability";
import { decodeQuestionDetails } from "../decoders/question_library";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestPath, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function questionPath(questionId: PublishedQuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}`;
}

function exactRevisionPath(publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple): string {
  if (
    !Number.isSafeInteger(publishedQuestionRevisionTuple.revisionNumber) ||
    publishedQuestionRevisionTuple.revisionNumber < 1 ||
    publishedQuestionRevisionTuple.revisionNumber > 4_294_967_295
  ) {
    throw new ApiProtocolError("Question Revision number must be one positive u32");
  }
  return `${questionPath(publishedQuestionRevisionTuple.publishedQuestionId)}/revisions/${encodeURIComponent(String(publishedQuestionRevisionTuple.revisionNumber))}`;
}

function questionAvailabilityEditNumberFromResponse(
  response: Response,
  path: string,
): QuestionAvailabilityEditNumber {
  return numberFromQuotedPositiveHeader(response, path, "Question Availability Edit Number");
}

async function questionJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST";
    readonly body?: unknown;
    readonly expectedQuestionAvailabilityEditNumber?: QuestionAvailabilityEditNumber;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.expectedQuestionAvailabilityEditNumber !== undefined) {
    headers["if-match"] = ifMatchHeaderForPositiveNumber(
      options.expectedQuestionAvailabilityEditNumber,
      path,
      "Question Availability Edit Number",
    );
  }
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
  publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  path: string,
): QuestionDetails {
  const actual = detail.summary.publishedQuestionRevisionTuple;
  if (
    actual.publishedQuestionId !== publishedQuestionRevisionTuple.publishedQuestionId ||
    actual.revisionNumber !== publishedQuestionRevisionTuple.revisionNumber
  ) {
    throw new ApiProtocolError(`API response ${path} does not match its exact Question Revision`);
  }
  return detail;
}

function availabilityTransition(
  body: Omit<QuestionAvailabilityTransition, "questionAvailabilityEditNumber"> & {
    readonly questionAvailabilityEditNumber: QuestionAvailabilityEditNumber;
  },
  response: Response,
  path: string,
): QuestionAvailabilityTransition {
  assertResponseMatchesPositiveNumber(
    response,
    body.questionAvailabilityEditNumber,
    path,
    "Question Availability Edit Number",
  );
  return body;
}

/** Composes Question availability independently from ordinary Question Library search. */
export function createQuestionAvailabilityClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionAvailabilityClient> {
  return {
    getQuestionLineage: async (questionId): Promise<LoadedQuestionLineage> => {
      const path = questionPath(questionId);
      const result = await questionJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionLineageView,
      );
      const { summary, viewerMayArchive } = result.body;
      if (summary.questionId !== questionId) {
        throw new ApiProtocolError(`API response ${path} does not match its Question lineage`);
      }
      return {
        summary,
        viewerMayArchive,
        questionAvailabilityEditNumber: questionAvailabilityEditNumberFromResponse(
          result.response,
          path,
        ),
      };
    },
    getQuestionRevision: async (publishedQuestionRevisionTuple): Promise<QuestionDetails> => {
      const path = exactRevisionPath(publishedQuestionRevisionTuple);
      const result = await questionJson(fetchImplementation, basePath, path, decodeQuestionDetails);
      return sameQuestionRevision(result.body, publishedQuestionRevisionTuple, path);
    },
    questionRevisionPreviewDocumentUrl: (publishedQuestionRevisionTuple) =>
      requestPath(
        basePath,
        `${exactRevisionPath(publishedQuestionRevisionTuple)}/preview-document`,
      ),
    archiveQuestion: async (
      questionId,
      confirmationTitle,
      expectedQuestionAvailabilityEditNumber,
    ): Promise<QuestionAvailabilityTransition> => {
      const path = `${questionPath(questionId)}/archive`;
      const result = await questionJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionAvailabilityTransition,
        {
          method: "POST",
          body: { confirmationTitle },
          expectedQuestionAvailabilityEditNumber,
        },
      );
      return availabilityTransition(result.body, result.response, path);
    },
    restoreQuestion: async (
      questionId,
      expectedQuestionAvailabilityEditNumber,
    ): Promise<QuestionAvailabilityTransition> => {
      const path = `${questionPath(questionId)}/restore`;
      const result = await questionJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionAvailabilityTransition,
        { method: "POST", expectedQuestionAvailabilityEditNumber },
      );
      return availabilityTransition(result.body, result.response, path);
    },
  };
}
