// Strict decoder for the answer-free receipt returned by Question Pool creation and fork.

import type { CreatedQuestionPool } from "../question_pool_creation";
import { DecodeError, decodePositiveInteger, decodeRecord } from "../decoder";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";

/** Decodes the server-issued identity and initial edit number for one ordinary Pool. */
export function decodeCreatedQuestionPool(value: unknown, path = "response"): CreatedQuestionPool {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolEditNumber"]);
  const questionPoolEditNumber = decodePositiveInteger(
    field(record, "questionPoolEditNumber", path),
    `${path}.questionPoolEditNumber`,
  );
  if (questionPoolEditNumber !== 1) {
    throw new DecodeError(`${path}.questionPoolEditNumber`, "Question Pool Edit Number 1");
  }
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: 1,
  };
}
