// Stable Questions-content conflict mapping; visible recovery is covered by the connected journey.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeAssignmentContentIssuedWorkConflict } from "../src/api/decoders.ts";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentIssuedWorkError,
  createHttpApiClient,
  resolveAssignmentContentSaveFailure,
} from "../src/api/http_client.ts";

const course = "0198e000-0000-7000-8000-000000000001";
const assignment = "0198e000-0000-7000-8000-000000000002";
const input = {
  title: "Peptide bond mastery",
  entries: [
    {
      kind: "fixed",
      questionId: "7K3-M9QP",
      position: 0,
      pointsPossible: "1",
      deliveryState: "active",
      scoringMode: "normal",
    },
  ],
};

function contentSave(response) {
  return createHttpApiClient({ fetch: async () => response }).saveAssignmentContent(
    course,
    assignment,
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

test("Questions content save gives issued learner work its own typed recovery", async () => {
  assert.deepEqual(decodeAssignmentContentIssuedWorkConflict({ kind: "issuedLearnerWork" }), {
    kind: "issuedLearnerWork",
  });
  await assert.rejects(
    contentSave(editorJsonResponse({ kind: "issuedLearnerWork" }, 409)),
    (error) => {
      assert.ok(error instanceof AssignmentIssuedWorkError);
      assert.equal(error.status, 409);
      assert.deepEqual(resolveAssignmentContentSaveFailure(error), {
        kind: "issuedLearnerWork",
        message:
          "Learner work has already been issued, so this assignment's question structure remains unchanged.",
      });
      return true;
    },
  );
});

test("Questions content conflict decoder rejects malformed and extra fields", () => {
  assert.throws(() => decodeAssignmentContentIssuedWorkConflict({ kind: "other" }), DecodeError);
  assert.throws(
    () => decodeAssignmentContentIssuedWorkConflict({ kind: "issuedLearnerWork", extra: true }),
    DecodeError,
  );
});

test("Questions content save treats malformed issued-work bodies as ordinary 409 errors", async () => {
  for (const body of [{ kind: "other" }, { kind: "issuedLearnerWork", extra: true }]) {
    await assert.rejects(contentSave(editorJsonResponse(body, 409)), (error) => {
      assert.ok(error instanceof ApiRequestError);
      assert.ok(!(error instanceof AssignmentIssuedWorkError));
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
      assert.ok(!(error instanceof AssignmentIssuedWorkError));
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
