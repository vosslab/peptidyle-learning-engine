import type { LearnerDisclosurePolicy } from "../../../generated/api/LearnerDisclosurePolicy";
import type { LearnerDisclosureTiming } from "../../../generated/api/LearnerDisclosureTiming";
import { decodeRecord, decodeStringEnum } from "../decoder";
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
