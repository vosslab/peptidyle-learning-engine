// Strict same-origin transport for one server-issued Published Question Pool.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import type { ApiClient } from "../client";
import { DecodeError, decodePositiveInteger, decodeRecord } from "../decoder";
import {
  decodeQuestionId,
  decodeQuestionRevisionTuple,
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

function requestMembers(input: CreateQuestionPoolInput): ReadonlyArray<QuestionRevisionTuple> {
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
  const tuples = new Set<string>();
  return input.members.map((member, index) => {
    const questionRevision = decodeQuestionRevisionTuple(member, `request.members[${index}]`, true);
    const key = `${questionRevision.questionId}:${questionRevision.revisionNumber}`;
    if (tuples.has(key)) {
      throw new ApiProtocolError(
        "Question Pool creation cannot include an exact revision more than once",
      );
    }
    tuples.add(key);
    return questionRevision;
  });
}

function decodeCreatedQuestionPool(value: unknown, path = "response"): CreatedQuestionPool {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolEditNumber"]);
  const questionPoolEditNumber = decodePositiveInteger(
    field(record, "questionPoolEditNumber", path),
    `${path}.questionPoolEditNumber`,
  );
  if (questionPoolEditNumber !== 1) {
    throw new DecodeError(`${path}.questionPoolEditNumber`, "Question Pool Edit Number 1");
  }
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: 1,
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
