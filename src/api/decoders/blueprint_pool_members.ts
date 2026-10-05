// Strict answer-free decoding for current Blueprint Assessment Pool membership.

import type { BlueprintPoolMembersView } from "../../../generated/api/BlueprintPoolMembersView";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionType } from "../../../generated/api/QuestionType";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import {
  DecodeError,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import {
  decodeBoundedArray,
  decodeQuestionId,
  decodePublishedQuestionRevisionTuple,
  field,
  requireOnlyFields,
} from "./shared";

export function decodeBlueprintPoolMembersView(
  value: unknown,
  path = "response",
): BlueprintPoolMembersView {
  // ASVS 1.5.2, 2.2.1/3: exact allowlisted shape, bounded unique Question IDs.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionPoolId",
    "questionPoolEditNumber",
    "disciplineUuid",
    "subjectUuid",
    "questionType",
    "backend",
    "members",
  ]);
  const questionPoolId = decodeQuestionId(
    field(record, "questionPoolId", path),
    `${path}.questionPoolId`,
  );
  const questionPoolEditNumber = decodePositiveInteger(
    field(record, "questionPoolEditNumber", path),
    `${path}.questionPoolEditNumber`,
  );
  const disciplineUuid = decodeUuid(
    field(record, "disciplineUuid", path),
    `${path}.disciplineUuid`,
  );
  const subjectUuid = decodeUuid(field(record, "subjectUuid", path), `${path}.subjectUuid`);
  const questionType = decodeStringEnum<QuestionType>(
    field(record, "questionType", path),
    `${path}.questionType`,
    [
      "multipleChoice",
      "multipleAnswer",
      "fillInBlank",
      "multipleFillInBlank",
      "numeric",
      "matching",
      "ordering",
      "hotspot",
    ],
  );
  const backend = decodeStringEnum<QuestionBackend>(
    field(record, "backend", path),
    `${path}.backend`,
    ["ple", "webwork", "imathas"],
  );
  const members = decodeBoundedArray(
    field(record, "members", path),
    `${path}.members`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    (member, memberPath) => decodePublishedQuestionRevisionTuple(member, memberPath, true),
  );
  if (
    members.length === 0 ||
    new Set(members.map((member) => member.publishedQuestionId)).size !== members.length
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
    disciplineUuid,
    subjectUuid,
    questionType,
    backend,
    members,
  };
}
