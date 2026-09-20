// Strict same-origin transport for Question lineage availability and exact revisions.

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
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
  numberFromResponseEtag,
} from "./conditional_request";
import {
  decodeQuestionAvailabilityTransition,
  decodeQuestionLineageView,
} from "../decoders/question_availability";
import { decodeQuestionDetails } from "../decoders/question_library";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestPath, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function questionPath(questionId: QuestionId): string {
  return `/api/questions/by-id/${encodedId(questionId)}`;
}

function exactRevisionPath(questionRevisionTuple: QuestionRevisionTuple): string {
  if (
    !Number.isSafeInteger(questionRevisionTuple.revisionNumber) ||
    questionRevisionTuple.revisionNumber < 1 ||
    questionRevisionTuple.revisionNumber > 4_294_967_295
  ) {
    throw new ApiProtocolError("Question Revision number must be one positive u32");
  }
  return `${questionPath(questionRevisionTuple.questionId)}/revisions/${encodeURIComponent(String(questionRevisionTuple.revisionNumber))}`;
}

function questionAvailabilityEditNumberFromResponse(
  response: Response,
  path: string,
): QuestionAvailabilityEditNumber {
  return numberFromResponseEtag(response, path, "Question Availability Edit Number");
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
  questionRevisionTuple: QuestionRevisionTuple,
  path: string,
): QuestionDetails {
  const actual = detail.summary.questionRevisionTuple;
  if (
    actual.questionId !== questionRevisionTuple.questionId ||
    actual.revisionNumber !== questionRevisionTuple.revisionNumber
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
    getQuestionRevision: async (questionRevisionTuple): Promise<QuestionDetails> => {
      const path = exactRevisionPath(questionRevisionTuple);
      const result = await questionJson(fetchImplementation, basePath, path, decodeQuestionDetails);
      return sameQuestionRevision(result.body, questionRevisionTuple, path);
    },
    questionRevisionPreviewDocumentUrl: (questionRevisionTuple) =>
      requestPath(basePath, `${exactRevisionPath(questionRevisionTuple)}/preview-document`),
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
