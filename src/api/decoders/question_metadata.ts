// Strict wire decoding for ordinary Published Question metadata receipts.

import type { SavedQuestionMetadata } from "../../../generated/api/SavedQuestionMetadata";
import { decodePositiveInteger, decodeRecord } from "../decoder";
import { decodePublishedQuestionRevisionTuple, field, requireOnlyFields } from "./shared";

export function decodeSavedQuestionMetadata(value: unknown): SavedQuestionMetadata {
  const path = "response";
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["publishedQuestionRevisionTuple", "metadataEditNumber"]);
  return {
    publishedQuestionRevisionTuple: decodePublishedQuestionRevisionTuple(
      field(record, "publishedQuestionRevisionTuple", path),
      `${path}.publishedQuestionRevisionTuple`,
      true,
    ),
    metadataEditNumber: decodePositiveInteger(
      field(record, "metadataEditNumber", path),
      `${path}.metadataEditNumber`,
    ),
  };
}
