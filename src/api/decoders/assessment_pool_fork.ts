// Strict browser decoders for Assessment-owned Question Pool fork operations.

import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { AssessmentQuestionPoolSelectionCountReceipt } from "../../../generated/api/AssessmentQuestionPoolSelectionCountReceipt";
import {
  DecodeError,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeUuid,
} from "../decoder";
import { decodeQuestionPoolMetadata, decodeQuestionPoolMemberView } from "./question_pool_library";
import { decodeBoundedArray, decodeQuestionId, field, requireOnlyFields } from "./shared";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { ImportedAssessmentQuestionPoolFork } from "../assessment_pool_fork";
import { decodeBloomClassificationView } from "./bloom_classification";

function positiveEditNumber(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Edit Number");
  }
  return decoded;
}

/** Decodes only exact current Pool members plus their narrow editor metadata. */
export function decodeAssessmentQuestionPoolForkView(
  value: unknown,
  path = "response",
): AssessmentQuestionPoolForkView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentEntryId",
    "questionPoolId",
    "questionPoolEditNumber",
    "selectionCount",
    "bloom",
    "metadata",
    "members",
  ]);
  const members = decodeBoundedArray(
    field(record, "members", path),
    `${path}.members`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    decodeQuestionPoolMemberView,
  );
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
    assessmentEntryId: decodeUuid(
      field(record, "assessmentEntryId", path),
      `${path}.assessmentEntryId`,
    ),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    bloom: decodeNullable(
      field(record, "bloom", path),
      `${path}.bloom`,
      decodeBloomClassificationView,
    ),
    members,
  };
}

/** Decodes the count-only command receipt without accepting a whole Assessment payload. */
export function decodeAssessmentQuestionPoolSelectionCountReceipt(
  value: unknown,
  path = "response",
): AssessmentQuestionPoolSelectionCountReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentEntryId", "selectionCount", "assessmentEditNumber"]);
  return {
    assessmentEntryId: decodeUuid(
      field(record, "assessmentEntryId", path),
      `${path}.assessmentEntryId`,
    ),
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    assessmentEditNumber: positiveEditNumber(
      field(record, "assessmentEditNumber", path),
      `${path}.assessmentEditNumber`,
    ),
  };
}

/** Decodes the atomic import receipt, never a browser-supplied Pool or Entry identity. */
export function decodeImportedAssessmentQuestionPoolFork(
  value: unknown,
  path = "response",
): ImportedAssessmentQuestionPoolFork {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentEntryId",
    "questionPoolId",
    "questionPoolEditNumber",
    "assessmentEditNumber",
  ]);
  return {
    assessmentEntryId: decodeUuid(
      field(record, "assessmentEntryId", path),
      `${path}.assessmentEntryId`,
    ),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    assessmentEditNumber: positiveEditNumber(
      field(record, "assessmentEditNumber", path),
      `${path}.assessmentEditNumber`,
    ),
  };
}
