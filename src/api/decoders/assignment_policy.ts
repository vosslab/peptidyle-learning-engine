import type { StudentDisclosurePolicy } from "../../../generated/api/StudentDisclosurePolicy";
import type { StudentDisclosureTiming } from "../../../generated/api/StudentDisclosureTiming";
import { decodeRecord, decodeStringEnum } from "../decoder";
import { field, requireOnlyFields } from "./shared";

/** Decode the exact assignment-owned Student disclosure matrix. */
export function decodeStudentDisclosurePolicy(
  value: unknown,
  path: string,
): StudentDisclosurePolicy {
  const record = decodeRecord(value, path);
  const fields = [
    "score",
    "per_item_correctness",
    "feedback_text",
    "solution",
    "class_statistics",
  ] as const;
  requireOnlyFields(record, path, fields);
  const decodeTiming = (fieldName: (typeof fields)[number]): StudentDisclosureTiming =>
    decodeStringEnum(field(record, fieldName, path), `${path}.${fieldName}`, [
      "during_attempt",
      "after_submit",
      "after_due",
      "after_close",
      "never",
    ] as const);
  return {
    score: decodeTiming("score"),
    per_item_correctness: decodeTiming("per_item_correctness"),
    feedback_text: decodeTiming("feedback_text"),
    solution: decodeTiming("solution"),
    class_statistics: decodeTiming("class_statistics"),
  };
}
