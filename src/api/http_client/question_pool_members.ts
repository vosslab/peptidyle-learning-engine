// Strict same-origin transport for ordinary Question Pool member replacement.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { QuestionPoolMembersClient, SavedQuestionPoolMembers } from "../question_pool_members";
import {
  decodeSavedQuestionPoolMembers,
  validateQuestionPoolMembersRequest,
} from "../decoders/question_pool_members";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";

function canonicalPoolId(value: QuestionPoolId): QuestionPoolId {
  const canonical = validateCanonicalQuestionIdSyntax(value);
  if (canonical === null || canonical !== value) {
    throw new ApiProtocolError("Question Pool ID must be canonical");
  }
  return canonical;
}

export function createQuestionPoolMembersClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionPoolMembersClient {
  return {
    saveQuestionPoolMembers: async (input): Promise<SavedQuestionPoolMembers> => {
      const request = validateQuestionPoolMembersRequest(input);
      const questionPoolId = canonicalPoolId(request.questionPoolId);
      const path = `/api/question-pools/${encodedId(questionPoolId)}/members`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        body: request,
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const receipt = decodeSavedQuestionPoolMembers(await boundedResponseJson(response, path));
      if (receipt.questionPoolId !== questionPoolId) {
        throw new ApiProtocolError("Question Pool member receipt does not match its request");
      }
      return receipt;
    },
  };
}
