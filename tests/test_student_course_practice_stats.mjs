import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentCourseResponseStats } from "../src/api/decoders/student_course_practice_stats.ts";
import { createStudentCourseResponseStatsClient } from "../src/api/http_client/student_course_practice_stats.ts";

const first = {
  publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
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

test("Response Stats accepts exact revisions, disclosed counts, and ranked results", () => {
  assert.deepEqual(decodeStudentCourseResponseStats({ questions: [first] }), {
    questions: [first],
  });
});

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
      questions: [
        {
          ...first,
          averageDisplayDurationMs: null,
          displayDurationSampleCount: 0,
        },
      ],
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

test("Response Stats client uses the exact Course endpoint and requires no-store", async () => {
  const calls = [];
  const client = createStudentCourseResponseStatsClient(async (input, init) => {
    calls.push([String(input), init]);
    return new Response(JSON.stringify({ questions: [first] }), {
      status: 200,
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  }, "");
  assert.deepEqual(await client.getStudentCourseResponseStats("CI6F2R8TA0"), {
    questions: [first],
  });
  assert.match(calls[0][0], /\/api\/student\/course-instances\/CI6F2R8TA0\/response-stats$/u);
  const cacheCheckingClient = createStudentCourseResponseStatsClient(
    async () =>
      new Response(JSON.stringify({ questions: [] }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    "",
  );
  await assert.rejects(
    cacheCheckingClient.getStudentCourseResponseStats("CI6F2R8TA0"),
    /no-store/u,
  );
});
