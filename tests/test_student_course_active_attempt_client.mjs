import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError } from "../src/api/http_client.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  decodeStudentCourseActiveAttempt,
  decodeStudentLatestFeedback,
} from "../src/api/decoders/live_student_course_landing.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE_INSTANCE_ID = "CI6F2R8TA0";
const ATTEMPT_ID = "00000000-0000-0000-0000-000000000001";

function response(body) {
  return new Response(JSON.stringify(body), {
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
    },
  });
}

test("Active Attempt decoder accepts an ID or explicit no-active state", () => {
  assert.deepEqual(
    decodeStudentCourseActiveAttempt({
      assessmentAttemptId: ATTEMPT_ID,
      startedAt: 1_786_000_000_000,
      latestActivityAt: 1_786_000_000_001,
    }),
    {
      assessmentAttemptId: ATTEMPT_ID,
      startedAt: 1_786_000_000_000,
      latestActivityAt: 1_786_000_000_001,
    },
  );
  assert.deepEqual(
    decodeStudentCourseActiveAttempt({
      assessmentAttemptId: null,
      startedAt: null,
      latestActivityAt: null,
    }),
    {
      assessmentAttemptId: null,
      startedAt: null,
      latestActivityAt: null,
    },
  );
  assert.throws(
    () => decodeStudentCourseActiveAttempt({ assessmentAttemptId: "not-an-attempt" }),
    /canonical Assessment Attempt UUID/u,
  );
  assert.throws(
    () =>
      decodeStudentCourseActiveAttempt({
        assessmentAttemptId: ATTEMPT_ID,
        startedAt: null,
        latestActivityAt: null,
      }),
    /Attempt ID with both timestamps/u,
  );
});

test("Latest Feedback decoder accepts only an Attempt ID or explicit empty state", () => {
  assert.deepEqual(decodeStudentLatestFeedback({ assessmentAttemptId: ATTEMPT_ID }), {
    assessmentAttemptId: ATTEMPT_ID,
  });
  assert.deepEqual(decodeStudentLatestFeedback({ assessmentAttemptId: null }), {
    assessmentAttemptId: null,
  });
  assert.throws(
    () => decodeStudentLatestFeedback({ assessmentAttemptId: "not-an-attempt" }),
    /canonical Assessment Attempt UUID/u,
  );
  assert.throws(
    () => decodeStudentLatestFeedback({ assessmentAttemptId: null, feedback: "hidden" }),
    /field allowed by this response contract/u,
  );
});

test("Active Attempt client reads the no-store Course-scoped shortcut projection", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    response({
      assessmentAttemptId: ATTEMPT_ID,
      startedAt: 1_786_000_000_000,
      latestActivityAt: 1_786_000_000_001,
    }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  const result = await client.getStudentCourseActiveAttempt(COURSE_INSTANCE_ID);

  assert.deepEqual(result, {
    assessmentAttemptId: ATTEMPT_ID,
    startedAt: 1_786_000_000_000,
    latestActivityAt: 1_786_000_000_001,
  });
  assert.equal(
    requests[0].url,
    `https://client.example.test/live/api/student/course-instances/${COURSE_INSTANCE_ID}/active-attempt`,
  );
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].credentials, "same-origin");
});

test("Active Attempt client rejects non-Course IDs before dispatch", () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    response({ assessmentAttemptId: null }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.throws(() => client.getStudentCourseActiveAttempt("BP7K3M2QAF"), ApiProtocolError);
  assert.equal(requests.length, 0);
});

test("Latest Feedback client uses its authenticated no-store shortcut route", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    response({ assessmentAttemptId: ATTEMPT_ID }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  assert.deepEqual(await client.getStudentLatestFeedback(), { assessmentAttemptId: ATTEMPT_ID });
  assert.equal(requests[0].url, "https://client.example.test/live/api/student/latest-feedback");
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
});
