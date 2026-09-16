// Strict same-origin transport for one server-issued Published Question Pool.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { ApiClient } from "../client";
import { DecodeError, decodePositiveInteger, decodeRecord } from "../decoder";
import {
  decodeQuestionId,
  decodeQuestionRevisionReference,
  field,
  requireOnlyFields,
} from "../decoders/shared";
import type {
  CreatedQuestionPool,
  CreateQuestionPoolInput,
  QuestionPoolCreationClient,
} from "../question_pool_creation";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { decodeQuestionPoolText } from "../decoders/question_pool_library";

const CREATE_QUESTION_POOL_PATH = "/api/question-pools";

function requestMembers(input: CreateQuestionPoolInput): ReadonlyArray<QuestionRevisionReference> {
  if (input.interchangeabilityAttested !== true) {
    throw new ApiProtocolError("Question Pool creation requires interchangeability attestation");
  }
  if (
    input.members.length === 0 ||
    input.members.length > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
  ) {
    throw new ApiProtocolError(
      "Question Pool creation requires a bounded nonempty Question selection",
    );
  }
  const references = new Set<string>();
  return input.members.map((member, index) => {
    const reference = decodeQuestionRevisionReference(member, `request.members[${index}]`, true);
    const key = `${reference.questionId}:${reference.revisionNumber}`;
    if (references.has(key)) {
      throw new ApiProtocolError(
        "Question Pool creation cannot include an exact revision more than once",
      );
    }
    references.add(key);
    return reference;
  });
}

function decodeCreatedQuestionPool(value: unknown, path = "response"): CreatedQuestionPool {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "revisionNumber"]);
  const revisionNumber = decodePositiveInteger(
    field(record, "revisionNumber", path),
    `${path}.revisionNumber`,
  );
  if (revisionNumber !== 1) {
    throw new DecodeError(`${path}.revisionNumber`, "Published Question Pool Revision 1");
  }
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    revisionNumber: 1,
  };
}

/** Composes the one create command without giving the browser Pool identity authority. */
export function createQuestionPoolCreationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof QuestionPoolCreationClient> {
  return {
    createQuestionPool: async (input): Promise<CreatedQuestionPool> => {
      const title = decodeQuestionPoolText(input.title, "request.title", 512);
      const description = decodeQuestionPoolText(input.description, "request.description", 4000);
      const members = requestMembers(input);
      const response = await requestSameOrigin(
        fetchImplementation,
        basePath,
        CREATE_QUESTION_POOL_PATH,
        {
          method: "POST",
          body: { title, description, members, interchangeabilityAttested: true },
        },
      );
      requireNoStore(response, CREATE_QUESTION_POOL_PATH);
      if (response.status !== 201) {
        throw new ApiRequestError(response.status, CREATE_QUESTION_POOL_PATH);
      }
      return decodeCreatedQuestionPool(
        await boundedResponseJson(response, CREATE_QUESTION_POOL_PATH),
      );
    },
  };
}
