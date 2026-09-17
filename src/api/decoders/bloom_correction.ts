// Strict correction receipts for exact Question and Pool Revisions.

import type { QuestionBloomCorrectionReceipt } from "../../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../../generated/api/QuestionPoolBloomCorrectionReceipt";
import { decodeRecord } from "../decoder";
import { decodeBloomClassificationView } from "./bloom_classification";
import { decodeQuestionPoolRevisionReference } from "./question_pool_library";
import { decodeQuestionRevisionReference, field, requireOnlyFields } from "./shared";

export function decodeQuestionBloomCorrectionReceipt(
  value: unknown,
  path: string,
): QuestionBloomCorrectionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionRevision", "bloom"]);
  return {
    questionRevision: decodeQuestionRevisionReference(
      field(record, "questionRevision", path),
      `${path}.questionRevision`,
    ),
    bloom: decodeBloomClassificationView(field(record, "bloom", path), `${path}.bloom`),
  };
}

export function decodeQuestionPoolBloomCorrectionReceipt(
  value: unknown,
  path: string,
): QuestionPoolBloomCorrectionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolRevision", "bloom"]);
  return {
    questionPoolRevision: decodeQuestionPoolRevisionReference(
      field(record, "questionPoolRevision", path),
      `${path}.questionPoolRevision`,
    ),
    bloom: decodeBloomClassificationView(field(record, "bloom", path), `${path}.bloom`),
  };
}
