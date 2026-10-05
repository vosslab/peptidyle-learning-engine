// Strict browser decoders for current published Question Pool detail.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionPoolMemberView } from "../../../generated/api/QuestionPoolMemberView";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { ReusableQuestionView } from "../../../generated/api/ReusableQuestionView";
import {
  DecodeError,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import { decodeQuestionSearchResult, decodeQuestionStatistics } from "./question_library";
import { decodeBloomClassificationView } from "./bloom_classification";
import {
  decodeBoundedArray,
  decodeAccountId,
  decodeQuestionId,
  decodePublishedQuestionRevisionTuple,
  field,
  requireOnlyFields,
} from "./shared";
import {
  decodePoolBackend,
  decodePoolLicense,
  decodePoolQuestionType,
  decodeQuestionPoolMetadata,
} from "./question_pool_summary";

function decodeReusableQuestionView(value: unknown, path: string): ReusableQuestionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "published_question_revision_tuple",
    "question_library",
    "selection_availability",
  ]);
  return {
    published_question_revision_tuple: decodePublishedQuestionRevisionTuple(
      field(record, "published_question_revision_tuple", path),
      `${path}.published_question_revision_tuple`,
    ),
    question_library: decodeQuestionSearchResult(
      field(record, "question_library", path),
      `${path}.question_library`,
    ),
    selection_availability: decodeStringEnum(
      field(record, "selection_availability", path),
      `${path}.selection_availability`,
      ["available", "retained"],
    ),
  };
}

/** Strictly decodes one ordered exact member for Pool and Assessment-fork readers. */
export function decodeQuestionPoolMemberView(value: unknown, path: string): QuestionPoolMemberView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["memberPosition", "publishedQuestionRevisionTuple", "question"]);
  return {
    memberPosition: decodeNonnegativeInteger(
      field(record, "memberPosition", path),
      `${path}.memberPosition`,
    ),
    publishedQuestionRevisionTuple: decodePublishedQuestionRevisionTuple(
      field(record, "publishedQuestionRevisionTuple", path),
      `${path}.publishedQuestionRevisionTuple`,
      true,
    ),
    question: decodeReusableQuestionView(field(record, "question", path), `${path}.question`),
  };
}

/** ASVS 1.5.2 and 2.2.3: validates exact pins and immutable member order. */
export function decodeQuestionPoolView(value: unknown, path = "response"): QuestionPoolView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionPoolId",
    "ownerAccountId",
    "questionType",
    "backend",
    "license",
    "questionPoolEditNumber",
    "metadata",
    "bloom",
    "members",
    "evidence",
  ]);
  const members = decodeBoundedArray(
    field(record, "members", path),
    `${path}.members`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    decodeQuestionPoolMemberView,
  );
  if (members.length === 0) {
    throw new DecodeError(`${path}.members`, "a nonempty published Question Pool");
  }
  for (const [index, member] of members.entries()) {
    if (member.memberPosition !== index) {
      throw new DecodeError(
        `${path}.members[${index}].memberPosition`,
        "its zero-based array position",
      );
    }
  }
  return {
    metadata: decodeQuestionPoolMetadata(field(record, "metadata", path), `${path}.metadata`),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    ownerAccountId: decodeAccountId(
      field(record, "ownerAccountId", path),
      `${path}.ownerAccountId`,
    ),
    questionType: decodePoolQuestionType(
      field(record, "questionType", path),
      `${path}.questionType`,
    ),
    backend: decodePoolBackend(field(record, "backend", path), `${path}.backend`),
    license: decodePoolLicense(field(record, "license", path), `${path}.license`),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    bloom: decodeNullable(
      field(record, "bloom", path),
      `${path}.bloom`,
      decodeBloomClassificationView,
    ),
    members,
    evidence: decodeQuestionStatistics(field(record, "evidence", path), `${path}.evidence`),
  };
}
