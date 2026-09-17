// Same-origin CAS transport for exact Revision-owned Bloom corrections.

import type { BloomClassificationCorrectionRequest } from "../../../generated/api/BloomClassificationCorrectionRequest";
import type { QuestionBloomCorrectionReceipt } from "../../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../../generated/api/QuestionPoolBloomCorrectionReceipt";
import type { QuestionPoolRevisionReference } from "../../../generated/api/QuestionPoolRevisionReference";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
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

function questionPath(reference: QuestionRevisionReference): string {
  const questionId = canonicalQuestionId(reference.questionId, "Question ID");
  if (
    !Number.isSafeInteger(reference.revisionNumber) ||
    reference.revisionNumber < 1 ||
    reference.revisionNumber > 4_294_967_295
  ) {
    throw new ApiProtocolError("Question Revision Number must be a positive u32 integer");
  }
  return `/api/questions/by-id/${encodedId(questionId)}/revisions/${reference.revisionNumber}/bloom`;
}

function poolPath(reference: QuestionPoolRevisionReference): string {
  const questionPoolId = canonicalQuestionId(reference.questionPoolId, "Question Pool ID");
  if (!Number.isSafeInteger(reference.revisionNumber) || reference.revisionNumber < 1) {
    throw new ApiProtocolError("Question Pool Revision Number must be a positive safe integer");
  }
  return `/api/question-pools/${encodedId(questionPoolId)}/revisions/${reference.revisionNumber}/bloom`;
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
    correctQuestionBloom: async (reference, request): Promise<QuestionBloomCorrectionReceipt> => {
      const path = questionPath(reference);
      const receipt = await post(
        fetchImplementation,
        basePath,
        path,
        request,
        decodeQuestionBloomCorrectionReceipt,
      );
      if (
        receipt.questionRevision.questionId !== reference.questionId ||
        receipt.questionRevision.revisionNumber !== reference.revisionNumber
      ) {
        throw new ApiProtocolError("Bloom correction receipt does not match its Question Revision");
      }
      return receipt;
    },
    correctQuestionPoolBloom: async (
      reference,
      request,
    ): Promise<QuestionPoolBloomCorrectionReceipt> => {
      const path = poolPath(reference);
      const receipt = await post(
        fetchImplementation,
        basePath,
        path,
        request,
        decodeQuestionPoolBloomCorrectionReceipt,
      );
      if (
        receipt.questionPoolRevision.questionPoolId !== reference.questionPoolId ||
        receipt.questionPoolRevision.revisionNumber !== reference.revisionNumber
      ) {
        throw new ApiProtocolError("Bloom correction receipt does not match its Pool Revision");
      }
      return receipt;
    },
  };
}
