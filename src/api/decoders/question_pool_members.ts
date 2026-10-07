// Closed request and receipt decoding for current Question Pool tuple-set saves.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { DecodeError, decodePositiveInteger, decodeRecord } from "../decoder";
import {
  decodePublishedQuestionRevisionTuple,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";
import type {
  SaveQuestionPoolMembersInput,
  SavedQuestionPoolMembers,
} from "../question_pool_members";

/** ASVS 1.5/2.2: bound the closed request, require positive exact tuples, and reject duplicate IDs. */
export function validateQuestionPoolMembersRequest(
  input: SaveQuestionPoolMembersInput,
): SaveQuestionPoolMembersInput {
  const expectedQuestionPoolEditNumber = decodePositiveInteger(
    input.expectedQuestionPoolEditNumber,
    "request.expectedQuestionPoolEditNumber",
  );
  if (
    input.members.length === 0 ||
    input.members.length > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
  ) {
    throw new DecodeError("request.members", "a bounded nonempty Question Pool tuple set");
  }
  const members = input.members.map((member, index) =>
    decodePublishedQuestionRevisionTuple(member, `request.members[${index}]`, true),
  );
  const questionIds = members.map((member) => member.publishedQuestionId);
  if (new Set(questionIds).size !== questionIds.length) {
    throw new DecodeError("request.members", "one Revision per Published Question");
  }
  return {
    questionPoolId: input.questionPoolId,
    expectedQuestionPoolEditNumber,
    members,
  };
}

/** Strictly accepts a Pool ID and positive Edit Number receipt. */
export function decodeSavedQuestionPoolMembers(
  value: unknown,
  path = "response",
): SavedQuestionPoolMembers {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolEditNumber"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
  };
}
