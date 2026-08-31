import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPoolPreview,
  decodeQuestionPoolPreviewRequest,
} from "../src/api/decoders.ts";
import {
  ApiRequestError,
  PreviewPlaneConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

const course = "C-12";
const assignment = "A-34";
const revision = "7";
const assignmentEntryId = "0198e000-0000-7000-8000-000000000017";

function previewResponse() {
  return {
    assignment,
    revision,
    assignmentEntryId,
    questionPoolLabel: "Pool 2",
    selectionCount: 1,
    selectionRule: { ordering: "randomized", algorithm: "v1" },
    candidates: [
      { questionId: "7K3-M9QP", title: "First candidate" },
      { questionId: "7K4-M9QP", title: "Second candidate" },
    ],
    selected: [{ questionId: "7K4-M9QP", title: "Second candidate" }],
  };
}

function jsonResponse(value, status = 200, cacheControl = "no-store") {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "cache-control": cacheControl },
  });
}

test("pool preview decoder accepts only the safe closed server projection", () => {
  const response = previewResponse();
  assert.deepEqual(decodeQuestionPoolPreview(response), response);
  assert.deepEqual(decodeQuestionPoolPreviewRequest({ assignmentEntryId }), { assignmentEntryId });
  assert.throws(
    () => decodeQuestionPoolPreviewRequest({ assignmentEntryId, seed: "browser" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionPoolPreview({
        ...response,
        selected: [{ questionId: "7K5-M9QP", title: "Not a candidate" }],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeQuestionPoolPreview({ ...response, answerKey: "not safe" }),
    DecodeError,
  );
});

test("pool preview transport sends only position with revision and requires no-store", async () => {
  const calls = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      calls.push({ input: String(input), init });
      return jsonResponse(previewResponse());
    },
  });
  const preview = await client.previewQuestionPool(course, assignment, revision, assignmentEntryId);
  assert.equal(preview.questionPoolLabel, "Pool 2");
  assert.equal(calls[0]?.input, "/api/courses/C-12/assignments/A-34/preview-pool-draw");
  assert.equal(calls[0]?.init?.method, "POST");
  assert.equal(calls[0]?.init?.headers?.["if-match"], '"7"');
  assert.equal(calls[0]?.init?.body, JSON.stringify({ assignmentEntryId }));
  assert.equal(calls[0]?.init?.credentials, "same-origin");
  assert.equal(calls[0]?.init?.cache, "no-store");
});

test("pool preview reports reload and unavailable recovery without response enumeration", async () => {
  const stale = createHttpApiClient({ fetch: async () => jsonResponse({}, 412) });
  await assert.rejects(
    stale.previewQuestionPool(course, assignment, revision, assignmentEntryId),
    PreviewPlaneConflictError,
  );
  const unavailable = createHttpApiClient({ fetch: async () => jsonResponse({}, 404) });
  await assert.rejects(
    unavailable.previewQuestionPool(course, assignment, revision, assignmentEntryId),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
});
