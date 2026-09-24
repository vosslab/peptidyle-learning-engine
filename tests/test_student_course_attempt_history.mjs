import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentCourseAttemptHistoryPage } from "../src/api/decoders/student_course_attempt_history.ts";
import { createStudentCourseAttemptHistoryClient } from "../src/api/http_client/student_course_attempt_history.ts";

function item(overrides = {}) {
  return {
    assessmentAttemptId: "00000000-0000-0000-0000-000000000002",
    assessmentId: "A5D9Q3XAH",
    assessmentTitle: "Peptide practice",
    assessmentAttemptNumber: 2,
    startedAt: 1_790_000_000_000,
    submittedAt: 1_790_000_060_000,
    assessmentScore: { pointsEarned: 8, pointsPossible: 10 },
    ...overrides,
  };
}

test("Attempt History accepts disclosed scores and a cursor page", () => {
  const value = {
    items: [
      item(),
      item({
        assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
        assessmentAttemptNumber: 1,
        startedAt: 1_780_000_000_000,
        submittedAt: 1_780_000_060_000,
        assessmentScore: undefined,
      }),
    ],
    nextCursor: "eyJhZnRlciI6InRlc3QifQ",
  };
  assert.deepEqual(decodeStudentCourseAttemptHistoryPage(value), {
    ...value,
    items: value.items.map(({ assessmentScore, ...entry }) => ({
      ...entry,
      ...(assessmentScore === undefined ? {} : { assessmentScore }),
    })),
  });
});

test("Attempt History rejects hidden fields, invalid scores, and wrong ordering", () => {
  for (const candidate of [
    { ...item(), timeZone: "America/Chicago" },
    { ...item(), assessmentScore: { pointsEarned: 1 } },
    { ...item(), assessmentScore: { pointsEarned: -1, pointsPossible: 2 } },
    { ...item(), submittedAt: undefined, assessmentScore: { pointsEarned: 1, pointsPossible: 2 } },
  ]) {
    assert.throws(() => decodeStudentCourseAttemptHistoryPage({ items: [candidate] }), DecodeError);
  }
  assert.throws(
    () =>
      decodeStudentCourseAttemptHistoryPage({
        items: [item({ startedAt: 1 }), item({ startedAt: 2 })],
      }),
    DecodeError,
  );
});

test("Attempt History client sends the bounded Course-scoped cursor request and requires no-store", async () => {
  const calls = [];
  const client = createStudentCourseAttemptHistoryClient(async (input, init) => {
    calls.push([String(input), init]);
    return new Response(JSON.stringify({ items: [item()] }), {
      status: 200,
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  }, "");
  await client.listStudentCourseAttemptHistory("CI6F2R8TA0", 25, "opaque-cursor");
  assert.match(calls[0][0], /pageSize=25/);
  assert.match(calls[0][0], /cursor=opaque-cursor/);
  await assert.rejects(client.listStudentCourseAttemptHistory("CI6F2R8TA0", 101), /page size/);
});
