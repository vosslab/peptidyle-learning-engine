// Strict answer-free decoding for current Blueprint Assessment Pool membership.

import type { BlueprintPoolMembersView } from "../../../generated/api/BlueprintPoolMembersView";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { DecodeError, decodePositiveInteger, decodeRecord } from "../decoder";
import {
  decodeBoundedArray,
  decodeQuestionId,
  decodeQuestionRevisionReference,
  field,
  requireOnlyFields,
} from "./shared";

export function decodeBlueprintPoolMembersView(
  value: unknown,
  path = "response",
): BlueprintPoolMembersView {
  // ASVS 1.5.2, 2.2.1/3: exact allowlisted shape, bounded unique Question IDs.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolEditNumber", "members"]);
  const questionPoolId = decodeQuestionId(
    field(record, "questionPoolId", path),
    `${path}.questionPoolId`,
  );
  const questionPoolEditNumber = decodePositiveInteger(
    field(record, "questionPoolEditNumber", path),
    `${path}.questionPoolEditNumber`,
  );
  const members = decodeBoundedArray(
    field(record, "members", path),
    `${path}.members`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    (member, memberPath) => decodeQuestionRevisionReference(member, memberPath, true),
  );
  if (
    members.length === 0 ||
    new Set(members.map((member) => member.questionId)).size !== members.length
  ) {
    throw new DecodeError(`${path}.members`, "nonempty Pool members with unique Question IDs");
  }
  for (const [index, member] of members.entries()) {
    if (member.revisionNumber > 2_147_483_647) {
      throw new DecodeError(
        `${path}.members[${index}].revisionNumber`,
        "a positive PostgreSQL integer Question Revision",
      );
    }
  }
  return {
    questionPoolId,
    questionPoolEditNumber,
    members,
  };
}
