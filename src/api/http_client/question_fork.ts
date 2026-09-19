// Strict same-origin transport for one server-created private Question Draft fork.

import type { ApiClient } from "../client";
import { DecodeError, decodeRecord, decodeUuid } from "../decoder";
import { decodeQuestionRevisionTuple, field, requireOnlyFields } from "../decoders/shared";
import type {
  ForkedPublishedQuestion,
  QuestionForkClient,
  QuestionForkIdempotencyKey,
} from "../question_fork";
import { parseDraftQuestionId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_QUESTION_REVISION_NUMBER = 4_294_967_295;

function exactForkPath(source: Parameters<QuestionForkClient["forkPublishedQuestion"]>[0]): string {
  const reference = decodeQuestionRevisionTuple(source, "request.source", true);
  if (reference.revisionNumber > MAX_QUESTION_REVISION_NUMBER) {
    throw new ApiProtocolError("Question Revision number must be one positive u32");
  }
  return `/api/questions/by-id/${encodeURIComponent(reference.questionId)}/revisions/${encodeURIComponent(String(reference.revisionNumber))}/fork`;
}

function idempotencyKey(value: QuestionForkIdempotencyKey, path: string): string {
  // ASVS 2.2.1, 4.2.5: the server accepts one UUID retry token, never an arbitrary header value.
  return decodeUuid(value, `API ${path} Idempotency-Key`);
}

function decodeForkedPublishedQuestion(value: unknown, path = "response"): ForkedPublishedQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["draftQuestion"]);
  const draftQuestion = field(record, "draftQuestion", path);
  if (typeof draftQuestion !== "string") {
    throw new DecodeError(`${path}.draftQuestion`, "a canonical private Draft UUID");
  }
  const parsed = parseDraftQuestionId(draftQuestion);
  if (parsed === null) {
    throw new DecodeError(`${path}.draftQuestion`, "a canonical private Draft UUID");
  }
  return { draftQuestion: parsed };
}

/** Composes the exact-revision fork command without browser authority over Draft content or lineage. */
export function createQuestionForkClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionForkClient> {
  return {
    forkPublishedQuestion: async (source, requestKey): Promise<ForkedPublishedQuestion> => {
      // ASVS 1.2.2, 2.2.1: validate and encode only the exact source path; send no fork payload.
      const path = exactForkPath(source);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        headers: { "idempotency-key": idempotencyKey(requestKey, path) },
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 201) {
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      }
      // ASVS 1.5.2: decode only the closed, answer-free navigation receipt.
      return decodeForkedPublishedQuestion(await boundedResponseJson(response, path));
    },
  };
}
