import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../../generated/api/StudentFeedbackReleaseTiming";
import { decodeRecord, decodeStringEnum } from "../decoder";
import { field, requireOnlyFields } from "./shared";

/** Decode the exact assignment-owned Student disclosure matrix. */
export function decodeStudentFeedbackReleaseRule(
  value: unknown,
  path: string,
): StudentFeedbackReleaseRule {
  const record = decodeRecord(value, path);
  const fields = [
    "score",
    "per_item_correctness",
    "question_feedback",
    "question_answer",
    "question_answer_explanation",
    "class_statistics",
  ] as const;
  requireOnlyFields(record, path, fields);
  const decodeTiming = (fieldName: (typeof fields)[number]): StudentFeedbackReleaseTiming =>
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
    question_feedback: decodeTiming("question_feedback"),
    question_answer: decodeTiming("question_answer"),
    question_answer_explanation: decodeTiming("question_answer_explanation"),
    class_statistics: decodeTiming("class_statistics"),
  };
}
