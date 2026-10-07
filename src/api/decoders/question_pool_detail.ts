// Strict browser decoders for current published Question Pool detail.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionPoolMemberView } from "../../../generated/api/QuestionPoolMemberView";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { ReusableQuestionView } from "../../../generated/api/ReusableQuestionView";
import {
  DecodeError,
  decodeBoolean,
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

/** Strictly decodes one exact member for Pool and Assessment-fork readers. */
export function decodeQuestionPoolMemberView(value: unknown, path: string): QuestionPoolMemberView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["publishedQuestionRevisionTuple", "question"]);
  return {
    publishedQuestionRevisionTuple: decodePublishedQuestionRevisionTuple(
      field(record, "publishedQuestionRevisionTuple", path),
      `${path}.publishedQuestionRevisionTuple`,
      true,
    ),
    question: decodeReusableQuestionView(field(record, "question", path), `${path}.question`),
  };
}

/** ASVS 1.5.2 and 2.2.3: validates exact pins and unordered membership. */
export function decodeQuestionPoolView(value: unknown, path = "response"): QuestionPoolView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionPoolId",
    "ownerAccountId",
    "canEditMetadata",
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
  const questionIds = members.map(
    (member) => member.publishedQuestionRevisionTuple.publishedQuestionId,
  );
  if (new Set(questionIds).size !== questionIds.length) {
    throw new DecodeError(`${path}.members`, "one Revision per Published Question");
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
    canEditMetadata: decodeBoolean(
      field(record, "canEditMetadata", path),
      `${path}.canEditMetadata`,
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
