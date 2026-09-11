// Focused browser-client proof for the Policies release-readiness action.

import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const course = "C-1";
const assignment = "A-2";

function createdWorkspace(displayTimeZone = "America/Chicago") {
  return {
    reference: assignment,
    editNumber: "1",
    status: "unreleased",
    title: "Peptide bonds",
    instructions: "",
    dueAt: null,
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {},
    studentFeedbackReleaseRule: {},
    displayTimeZone,
    questions: [],
  };
}

test("Assignment Workspace accepts exact browser-supported zones and rejects invalid names", async () => {
  const validFetch = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace("UTC")), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  const created = await createHttpApiClient({
    fetch: validFetch.recordingFetch,
  }).createLiveAssignment(course, { title: "Peptide bonds", instructions: "" });
  assert.equal(created.workspace.displayTimeZone, "UTC");

  const invalidFetch = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace("not/a-zone")), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  await assert.rejects(
    createHttpApiClient({ fetch: invalidFetch.recordingFetch }).createLiveAssignment(course, {
      title: "Peptide bonds",
      instructions: "",
    }),
    /response\.displayTimeZone must be a browser-supported IANA time-zone name/u,
  );
});

test("Assignment creation uses the Course Instance assignment boundary", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace()), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  await createHttpApiClient({ fetch: recordingFetch }).createLiveAssignment(course, {
    title: "Peptide bonds",
    instructions: "",
  });

  assert.equal(requests[0].method, "POST");
  assert.equal(new URL(requests[0].url).pathname, "/api/course-instances/C-1/assignments");
});

test("release readiness uses the current direct Assignment validation boundary", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify({ canRelease: false, issues: ["noPublishedQuestions"] }), {
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
        },
      }),
  );

  const validation = await createHttpApiClient({
    fetch: recordingFetch,
  }).validateLiveAssignmentRelease(course, assignment);

  assert.deepEqual(validation, { canRelease: false, issues: ["noPublishedQuestions"] });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "GET");
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/C-1/assignments/A-2/release-validation",
  );
});
