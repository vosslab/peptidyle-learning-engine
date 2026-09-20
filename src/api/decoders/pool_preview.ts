// Strict decoder for an Instructor Question Pool Preview.

import type { QuestionPoolPreview } from "../../../generated/api/QuestionPoolPreview";
import type { QuestionPoolPreviewItem } from "../../../generated/api/QuestionPoolPreviewItem";
import {
  DecodeError,
  decodeArray,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";

function decodeAssessmentEditNumber(value: unknown, path: string): string {
  const assessmentEditNumber = decodeString(value, path);
  if (
    !/^[1-9][0-9]*$/u.test(assessmentEditNumber) ||
    BigInt(assessmentEditNumber) > 9_223_372_036_854_775_807n
  ) {
    throw new DecodeError(path, "a positive Assessment Edit Number");
  }
  return assessmentEditNumber;
}

function decodePreviewItem(value: unknown, path: string): QuestionPoolPreviewItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "questionTitle"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionTitle: decodeString(field(record, "questionTitle", path), `${path}.questionTitle`),
  };
}

/** Decode one no-store preview and reject leftover generic clocks. */
export function decodeQuestionPoolPreview(value: unknown, path = "response"): QuestionPoolPreview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentId",
    "assessmentEditNumber",
    "assessmentEntryId",
    "questionPoolLabel",
    "selectionCount",
    "selectionRule",
    "items",
    "selectedItems",
  ]);
  const selectionRuleRecord = decodeRecord(
    field(record, "selectionRule", path),
    `${path}.selectionRule`,
  );
  requireOnlyFields(selectionRuleRecord, `${path}.selectionRule`, ["selectedQuestionOrder"]);
  const selectedQuestionOrder = decodeString(
    field(selectionRuleRecord, "selectedQuestionOrder", `${path}.selectionRule`),
    `${path}.selectionRule.selectedQuestionOrder`,
  );
  if (selectedQuestionOrder !== "questionPoolOrder" && selectedQuestionOrder !== "randomOrder") {
    throw new DecodeError(
      `${path}.selectionRule.selectedQuestionOrder`,
      "questionPoolOrder or randomOrder",
    );
  }
  return {
    assessmentId: decodeString(field(record, "assessmentId", path), `${path}.assessmentId`),
    assessmentEditNumber: decodeAssessmentEditNumber(
      field(record, "assessmentEditNumber", path),
      `${path}.assessmentEditNumber`,
    ),
    assessmentEntryId: decodeString(
      field(record, "assessmentEntryId", path),
      `${path}.assessmentEntryId`,
    ),
    questionPoolLabel: decodeString(
      field(record, "questionPoolLabel", path),
      `${path}.questionPoolLabel`,
    ),
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    selectionRule: { selectedQuestionOrder },
    items: decodeArray(field(record, "items", path), `${path}.items`, decodePreviewItem),
    selectedItems: decodeArray(
      field(record, "selectedItems", path),
      `${path}.selectedItems`,
      decodePreviewItem,
    ),
  };
}
