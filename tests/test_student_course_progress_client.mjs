import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError } from "../src/api/http_client.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE_INSTANCE_ID = "CI6F2R8TA0";

function progressResponse() {
  return {
    assessments: [
      {
        id: "A5D9Q3XAH",
        title: "Peptide practice",
        assessmentType: "regular_assignment",
        assessmentAttemptCount: 2,
        submittedAssessmentAttemptCount: 1,
        latestAssessmentAttemptNumber: 2,
        latestAssessmentAttemptCompletion: "inProgress",
        latestActivityAt: 1_786_000_000_000,
      },
    ],
  };
}

function response(body) {
  return new Response(JSON.stringify(body), {
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
    },
  });
}

test("Course Progress client reads the authenticated Course-scoped projection without caching", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    response(progressResponse()),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  const result = await client.getStudentCourseProgress(COURSE_INSTANCE_ID);

  assert.deepEqual(result, progressResponse().assessments);
  assert.equal(
    requests[0].url,
    `https://client.example.test/live/api/student/course-instances/${COURSE_INSTANCE_ID}/progress`,
  );
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].credentials, "same-origin");
});

test("Course Progress client rejects non-Course IDs before dispatch", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    response(progressResponse()),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.throws(() => client.getStudentCourseProgress("BP7K3M2QAF"), ApiProtocolError);
  assert.equal(requests.length, 0);
});
