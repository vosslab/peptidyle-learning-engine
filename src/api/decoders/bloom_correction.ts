// Strict correction receipts for Question Revisions and current Question Pools.

import type { QuestionBloomCorrectionReceipt } from "../../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../../generated/api/QuestionPoolBloomCorrectionReceipt";
import { decodeBloomClassificationView } from "./bloom_classification";
import { decodeRecord } from "../decoder";
import {
  decodeQuestionId,
  decodePublishedQuestionRevisionTuple,
  field,
  requireOnlyFields,
} from "./shared";

export function decodeQuestionBloomCorrectionReceipt(
  value: unknown,
  path: string,
): QuestionBloomCorrectionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["publishedQuestionRevisionTuple", "bloom"]);
  return {
    publishedQuestionRevisionTuple: decodePublishedQuestionRevisionTuple(
      field(record, "publishedQuestionRevisionTuple", path),
      `${path}.publishedQuestionRevisionTuple`,
    ),
    bloom: decodeBloomClassificationView(field(record, "bloom", path), `${path}.bloom`),
  };
}

export function decodeQuestionPoolBloomCorrectionReceipt(
  value: unknown,
  path: string,
): QuestionPoolBloomCorrectionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "bloom"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    bloom: decodeBloomClassificationView(field(record, "bloom", path), `${path}.bloom`),
  };
}
