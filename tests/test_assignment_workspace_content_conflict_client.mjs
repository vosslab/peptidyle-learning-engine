// Stable Questions-content conflict mapping; visible recovery is covered by the connected journey.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeSuccessorAssignmentRevisionRequired } from "../src/api/decoders.ts";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentSuccessorRevisionRequiredError,
  createHttpApiClient,
  resolveAssignmentContentSaveFailure,
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
    },
  ],
};

function contentSave(response) {
  return createHttpApiClient({ fetch: async () => response }).saveAssignmentContent(
    course,
    assignment,
    "A-1",
    input,
    '"1"',
  );
}

function editorJsonResponse(value, status) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Questions content save binds the reviewed Assignment Edit Number", async () => {
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
  assert.deepEqual(await request.json(), {
    ...input,
    baseEditNumber: "1",
  });
});

const successorRevisionRequirement = {
  baseRevision: { assignment: "A-1", revision_number: "1" },
};

test("Questions content save gives issued Student work a successor-revision recovery", async () => {
  assert.deepEqual(decodeSuccessorAssignmentRevisionRequired(successorRevisionRequirement), {
    baseRevision: { assignment: "A-1", revision_number: "1" },
  });
  await assert.rejects(
    contentSave(editorJsonResponse(successorRevisionRequirement, 409)),
    (error) => {
      assert.ok(error instanceof AssignmentSuccessorRevisionRequiredError);
      assert.equal(error.status, 409);
      assert.deepEqual(error.requirement, successorRevisionRequirement);
      assert.deepEqual(resolveAssignmentContentSaveFailure(error), {
        kind: "successorRevisionRequired",
        message:
          "Student work already pins this Assignment Revision. Create a successor Draft Assignment Revision for structural question changes.",
      });
      return true;
    },
  );
});

test("Successor-revision decoder rejects malformed and extra fields", () => {
  assert.throws(() => decodeSuccessorAssignmentRevisionRequired({}), DecodeError);
  assert.throws(
    () =>
      decodeSuccessorAssignmentRevisionRequired({
        ...successorRevisionRequirement,
        extra: true,
      }),
    DecodeError,
  );
});

test("Questions content save treats malformed successor-revision bodies as ordinary 409 errors", async () => {
  for (const body of [{}, { ...successorRevisionRequirement, extra: true }]) {
    await assert.rejects(contentSave(editorJsonResponse(body, 409)), (error) => {
      assert.ok(error instanceof ApiRequestError);
      assert.ok(!(error instanceof AssignmentSuccessorRevisionRequiredError));
      assert.equal(error.status, 409);
      return true;
    });
  }
});

test("Questions content save keeps stale revisions and ordinary conflicts distinct", async () => {
  await assert.rejects(
    contentSave(editorJsonResponse({ error: "record changed" }, 409)),
    (error) => {
      assert.ok(error instanceof ApiRequestError);
      assert.ok(!(error instanceof AssignmentSuccessorRevisionRequiredError));
      assert.equal(resolveAssignmentContentSaveFailure(error).kind, "retryable");
      return true;
    },
  );
  await assert.rejects(
    contentSave(editorJsonResponse({ error: "assignment changed" }, 412)),
    (error) => {
      assert.ok(error instanceof AssignmentConflictError);
      assert.equal(resolveAssignmentContentSaveFailure(error).kind, "staleRevision");
      return true;
    },
  );
});
