// Same-origin CAS transport for exact Revision-owned Bloom corrections.

import type { BloomClassificationCorrectionRequest } from "../../../generated/api/BloomClassificationCorrectionRequest";
import type { QuestionBloomCorrectionReceipt } from "../../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../../generated/api/QuestionPoolBloomCorrectionReceipt";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";
import type { BloomClassificationCorrectionClient } from "../bloom_classification";
import { decodeBloomClassificationCorrectionRequest } from "../decoders/bloom_classification";
import {
  decodeQuestionBloomCorrectionReceipt,
  decodeQuestionPoolBloomCorrectionReceipt,
} from "../decoders/bloom_correction";
import { ApiProtocolError, ApiRequestError, BloomClassificationConflictError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function canonicalQuestionId(value: string, label: string): string {
  const canonical = validateCanonicalQuestionIdSyntax(value);
  if (canonical === null || canonical !== value) {
    throw new ApiProtocolError(`${label} must be canonical`);
  }
  return canonical;
}

function questionPath(questionRevision: QuestionRevisionTuple): string {
  const questionId = canonicalQuestionId(questionRevision.questionId, "Question ID");
  if (
    !Number.isSafeInteger(questionRevision.revisionNumber) ||
    questionRevision.revisionNumber < 1 ||
    questionRevision.revisionNumber > 4_294_967_295
  ) {
    throw new ApiProtocolError("Question Revision Number must be a positive u32 integer");
  }
  return `/api/questions/by-id/${encodedId(questionId)}/revisions/${questionRevision.revisionNumber}/bloom`;
}

function poolPath(questionPoolId: QuestionId): string {
  const canonical = canonicalQuestionId(questionPoolId, "Question Pool ID");
  return `/api/question-pools/${encodedId(canonical)}/bloom`;
}

async function post<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  request: BloomClassificationCorrectionRequest,
  decode: (value: unknown, path: string) => T,
): Promise<T> {
  // ASVS 1.5.2/2.2.1: validate the complete pair and precision-safe CAS
  // precondition before dispatching the same-origin JSON mutation.
  const body = decodeBloomClassificationCorrectionRequest(request, "request");
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body,
  });
  requireNoStore(response, path);
  // ASVS 15.4.2: a stale response is surfaced to the editor exactly once;
  // this transport never retries or merges a correction.
  if (response.status === 412) throw new BloomClassificationConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decode(await boundedResponseJson(response, path), "response");
}

export function createBloomClassificationCorrectionClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): BloomClassificationCorrectionClient {
  return {
    correctQuestionBloom: async (questionRevision, request): Promise<QuestionBloomCorrectionReceipt> => {
      const path = questionPath(questionRevision);
      const receipt = await post(
        fetchImplementation,
        basePath,
        path,
        request,
        decodeQuestionBloomCorrectionReceipt,
      );
      if (
        receipt.questionRevision.questionId !== questionRevision.questionId ||
        receipt.questionRevision.revisionNumber !== questionRevision.revisionNumber
      ) {
        throw new ApiProtocolError("Bloom correction receipt does not match its Question Revision");
      }
      return receipt;
    },
    correctQuestionPoolBloom: async (
      questionPoolId,
      request,
    ): Promise<QuestionPoolBloomCorrectionReceipt> => {
      const path = poolPath(questionPoolId);
      const receipt = await post(
        fetchImplementation,
        basePath,
        path,
        request,
        decodeQuestionPoolBloomCorrectionReceipt,
      );
      if (receipt.questionPoolId !== questionPoolId) {
        throw new ApiProtocolError("Bloom correction receipt does not match its Question Pool");
      }
      return receipt;
    },
  };
}
