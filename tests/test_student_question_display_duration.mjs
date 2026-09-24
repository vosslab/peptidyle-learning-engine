import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentQuestionDisplayDurationCheckpoint } from "../src/api/decoders/student_question_display_duration.ts";
import { createStudentQuestionDisplayDurationClient } from "../src/api/http_client/student_question_display_duration.ts";

const attemptId = "00000000-0000-0000-0000-000000000001";
const receipt = {
  assessmentAttemptId: attemptId,
  position: 2,
  cumulativeDisplayDurationMs: 1_250,
};

test("display-duration receipt is strict and requires cumulative safe milliseconds", () => {
  assert.deepEqual(decodeStudentQuestionDisplayDurationCheckpoint(receipt), receipt);
  for (const value of [
    { ...receipt, cumulativeDisplayDurationMs: -1 },
    { ...receipt, cumulativeDisplayDurationMs: Number.MAX_SAFE_INTEGER + 1 },
    { ...receipt, timeZone: "America/Chicago" },
  ]) {
    assert.throws(() => decodeStudentQuestionDisplayDurationCheckpoint(value), DecodeError);
  }
});

test("checkpoint client sends a Course-owned Attempt position and monotone total", async () => {
  const calls = [];
  const client = createStudentQuestionDisplayDurationClient(async (input, init) => {
    calls.push([String(input), init]);
    return new Response(JSON.stringify(receipt), {
      status: 200,
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  }, "");
  assert.deepEqual(
    await client.checkpointStudentQuestionDisplayDuration(attemptId, 2, 1_000),
    receipt,
  );
  assert.match(calls[0][0], /\/api\/assessment-attempts\/.*\/questions\/2\/display-duration$/u);
  assert.equal(calls[0][1]?.method, "PUT");
  assert.deepEqual(JSON.parse(String(calls[0][1]?.body)), { cumulativeDisplayDurationMs: 1_000 });
  await assert.rejects(
    client.checkpointStudentQuestionDisplayDuration(attemptId, 2, 1_500),
    /does not match/u,
  );
});
