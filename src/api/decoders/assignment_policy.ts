import { MAX_ASSIGNMENT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSIGNMENT_TIME_LIMIT_SECONDS";
import type { AssignmentRunTiming } from "../../../generated/api/AssignmentRunTiming";
import type { LearnerDisclosurePolicy } from "../../../generated/api/LearnerDisclosurePolicy";
import type { LearnerDisclosureTiming } from "../../../generated/api/LearnerDisclosureTiming";
import {
  DecodeError,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

/** Decode the exact assignment-owned learner disclosure matrix. */
export function decodeLearnerDisclosurePolicy(
  value: unknown,
  path: string,
): LearnerDisclosurePolicy {
  const record = decodeRecord(value, path);
  const fields = [
    "score",
    "perItemCorrectness",
    "feedbackText",
    "solution",
    "classStatistics",
  ] as const;
  requireOnlyFields(record, path, fields);
  const decodeTiming = (fieldName: (typeof fields)[number]): LearnerDisclosureTiming =>
    decodeStringEnum(field(record, fieldName, path), `${path}.${fieldName}`, [
      "duringAttempt",
      "afterSubmit",
      "afterDue",
      "afterClose",
      "never",
    ] as const);
  return {
    score: decodeTiming("score"),
    perItemCorrectness: decodeTiming("perItemCorrectness"),
    feedbackText: decodeTiming("feedbackText"),
    solution: decodeTiming("solution"),
    classStatistics: decodeTiming("classStatistics"),
  };
}

export function decodeAssignmentRunTiming(value: unknown, path: string): AssignmentRunTiming {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeLimitSeconds"]);
  const timeLimitSeconds = decodeNullable(
    field(record, "timeLimitSeconds", path),
    `${path}.timeLimitSeconds`,
    (seconds, secondsPath) => {
      const decoded = decodePositiveInteger(seconds, secondsPath);
      if (decoded > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS) {
        throw new DecodeError(
          secondsPath,
          `a positive whole-second limit no greater than ${MAX_ASSIGNMENT_TIME_LIMIT_SECONDS}`,
        );
      }
      return decoded;
    },
  );
  return { timeLimitSeconds };
}
