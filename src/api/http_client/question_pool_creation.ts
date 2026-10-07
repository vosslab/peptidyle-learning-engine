// Strict same-origin transport for one server-issued Published Question Pool.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { ApiClient } from "../client";
import { DecodeError } from "../decoder";
import { decodePublishedQuestionRevisionTuple } from "../decoders/shared";
import { decodeCreatedQuestionPool } from "../decoders/question_pool_creation";
import type {
  CreatedQuestionPool,
  CreateQuestionPoolInput,
  QuestionPoolCreationClient,
} from "../question_pool_creation";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { decodeQuestionPoolText } from "../decoders/question_pool_summary";

const CREATE_QUESTION_POOL_PATH = "/api/question-pools";

function requestMembers(
  input: CreateQuestionPoolInput,
): ReadonlyArray<PublishedQuestionRevisionTuple> {
  if (
    input.members.length === 0 ||
    input.members.length > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
  ) {
    throw new ApiProtocolError(
      "Question Pool creation requires a bounded nonempty Question selection",
    );
  }
  const tuples = new Set<string>();
  const members = input.members.map((member, index) => {
    const publishedQuestionRevisionTuple = decodePublishedQuestionRevisionTuple(
      member,
      `request.members[${index}]`,
      true,
    );
    const key = `${publishedQuestionRevisionTuple.publishedQuestionId}:${publishedQuestionRevisionTuple.revisionNumber}`;
    if (tuples.has(key)) {
      throw new ApiProtocolError("Question Pool creation cannot repeat a Question");
    }
    tuples.add(key);
    return publishedQuestionRevisionTuple;
  });
  const questionIds = new Set(members.map((member) => member.publishedQuestionId));
  if (questionIds.size !== members.length) {
    throw new ApiProtocolError("Question Pool creation allows one Revision per Question");
  }
  return members;
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
      const tags =
        input.tags === undefined
          ? undefined
          : input.tags.map((tag, index) =>
              decodeQuestionPoolText(tag, `request.tags[${index}]`, 120),
            );
      if (tags !== undefined && new Set(tags).size !== tags.length) {
        throw new DecodeError("request.tags", "unique tags");
      }
      const response = await requestSameOrigin(
        fetchImplementation,
        basePath,
        CREATE_QUESTION_POOL_PATH,
        {
          method: "POST",
          body: {
            title,
            description,
            members,
            ...(tags === undefined ? {} : { tags }),
          },
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
