import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentCourseResponseStats } from "../src/api/decoders/student_course_response_stats.ts";

const first = {
  publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
  questionTitle: "Peptide bond",
  fullCreditAttemptCount: 1,
  partialCreditAttemptCount: 2,
  incorrectAttemptCount: 1,
  unansweredAttemptCount: 0,
  disclosedAttemptCount: 4,
  notFullCreditCount: 3,
  averageDisplayDurationMs: 12_500,
  displayDurationSampleCount: 2,
  relevantAssessmentAttemptId: "00000000-0000-0000-0000-000000000002",
};

test("Response Stats rejects hidden fields and inconsistent or unranked counts", () => {
  for (const candidate of [
    { ...first, correctAnswer: "hidden" },
    { ...first, timeZone: "America/Chicago" },
    { ...first, notFullCreditCount: 2 },
    { ...first, disclosedAttemptCount: 0 },
    { ...first, displayDurationSampleCount: 0 },
    { ...first, averageDisplayDurationMs: null },
    { ...first, averageDisplayDurationMs: -1 },
    {
      ...first,
      publishedQuestionRevisionTuple: { publishedQuestionId: "invalid", revisionNumber: 2 },
    },
  ]) {
    assert.throws(() => decodeStudentCourseResponseStats({ questions: [candidate] }), DecodeError);
  }
  assert.deepEqual(
    decodeStudentCourseResponseStats({
      questions: [{ ...first, averageDisplayDurationMs: null, displayDurationSampleCount: 0 }],
    }).questions[0]?.averageDisplayDurationMs,
    null,
  );
  assert.throws(
    () =>
      decodeStudentCourseResponseStats({
        questions: [
          first,
          {
            ...first,
            publishedQuestionRevisionTuple: { publishedQuestionId: "2R5X-E7YA", revisionNumber: 1 },
            notFullCreditCount: 4,
            disclosedAttemptCount: 4,
            partialCreditAttemptCount: 3,
            incorrectAttemptCount: 1,
            fullCreditAttemptCount: 0,
          },
        ],
      }),
    DecodeError,
  );
});
