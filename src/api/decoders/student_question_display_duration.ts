// Strict decoder for the monotone Question display-duration checkpoint receipt.

import type { StudentQuestionDisplayDurationCheckpoint } from "../student_question_display_duration";
import {
  DecodeError,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import { field, requireOnlyFields } from "./shared";

export function decodeStudentQuestionDisplayDurationCheckpoint(
  value: unknown,
  path = "response",
): StudentQuestionDisplayDurationCheckpoint {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttemptId",
    "position",
    "cumulativeDisplayDurationMs",
  ]);
  const rawAttemptId = decodeString(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
  );
  const assessmentAttemptId = parseAssessmentAttemptId(rawAttemptId);
  if (assessmentAttemptId === null) {
    throw new DecodeError(`${path}.assessmentAttemptId`, "an Assessment Attempt UUID");
  }
  return {
    assessmentAttemptId,
    position: decodePositiveInteger(field(record, "position", path), `${path}.position`),
    cumulativeDisplayDurationMs: decodeNonnegativeInteger(
      field(record, "cumulativeDisplayDurationMs", path),
      `${path}.cumulativeDisplayDurationMs`,
    ),
  };
}
