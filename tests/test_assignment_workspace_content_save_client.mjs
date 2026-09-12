// Assignment content saves use the current Assignment's edit number and lifecycle state.

import assert from "node:assert/strict";
import test from "node:test";

import {
  ApiRequestError,
  AssignmentConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const course = "0198e000-0000-7000-8000-000000000001";
const assignment = "0198e000-0000-7000-8000-000000000002";
const input = {
  title: "Peptide bond mastery",
  entries: [
    {
      kind: "fixedQuestion",
      questionId: "7K3-M9QP",
      pointsPossible: "1",
      availability: "available",
      scoringRule: "normal",
      questionAttemptLimit: { maxAttempts: null },
      questionAttemptTimeLimit: { kind: "unlimited" },
    },
  ],
};

function contentSave(response) {
  const client = createHttpApiClient({ fetch: async () => response });
  return client.saveAssignmentContent(course, assignment, "A-1", input, '"1"');
}

function editorJsonResponse(value, status) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Assignment content save binds the reviewed Assignment Edit Number", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    editorJsonResponse({ error: "assignment changed" }, 412),
  );

  await assert.rejects(
    createHttpApiClient({ fetch: recordingFetch }).saveAssignmentContent(
      course,
      assignment,
      "A-1",
      input,
      '"1"',
    ),
    AssignmentConflictError,
  );

  assert.equal(requests.length, 1);
  const request = requests[0];
  assert.equal(request.headers.get("if-match"), '"1"');
  assert.deepEqual(await request.json(), { ...input, baseEditNumber: "1" });
});

test("Assignment content save reports a stale edit number as 412", async () => {
  await assert.rejects(
    contentSave(editorJsonResponse({ error: "assignment changed" }, 412)),
    (error) => {
      assert.ok(error instanceof AssignmentConflictError);
      assert.equal(error.status, 412);
      return true;
    },
  );
});

test("Assignment content save preserves lifecycle conflict and validation statuses", async () => {
  await assert.rejects(contentSave(editorJsonResponse({ error: "not released" }, 409)), (error) => {
    assert.ok(error instanceof AssignmentConflictError);
    assert.equal(error.status, 409);
    return true;
  });
  await assert.rejects(
    contentSave(editorJsonResponse({ error: "invalid content" }, 422)),
    (error) => {
      assert.ok(error instanceof ApiRequestError);
      assert.equal(error.status, 422);
      return true;
    },
  );
});
