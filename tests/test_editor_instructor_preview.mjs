// test_editor_instructor_preview.mjs - permanent behavior gates for the explicit author-preview boundary.

import assert from "node:assert/strict";
import test from "node:test";

import {
  InstructorPreviewConflictError,
  InstructorPreviewRequestError,
  createInstructorPreviewClient,
  decodeInstructorPreview,
} from "../src/pages/editor_instructor_preview.ts";

const workspace = "0198e000-0000-7000-8000-000000000010";
const response = { kind: "numeric", tolerance: { kind: "exact" }, unit: null };
const presentation = {
  kind: "available",
  title: "Peptide-bond geometry",
  prompt: [{ kind: "text", markdown: "Estimate the omega angle." }],
  response,
  seed: 17,
  correctResponse: [{ kind: "text", markdown: "180 degrees." }],
  rationale: [{ kind: "text", markdown: "The peptide bond is planar." }],
};

test("author preview accepts only an exact display-ready presentation", () => {
  const decoded = decodeInstructorPreview(presentation, workspace);
  assert.deepEqual(decoded, {
    kind: "available",
    presentation: {
      title: presentation.title,
      prompt: presentation.prompt,
      response: presentation.response,
      seed: presentation.seed,
      correctResponse: presentation.correctResponse,
      rationale: presentation.rationale,
    },
  });
  assert.equal("workspace" in decoded.presentation, false);

  for (const field of ["key", "answerKey", "grading", "score", "source", "provider"]) {
    assert.throws(
      () => decodeInstructorPreview({ ...presentation, [field]: "forbidden" }, workspace),
      /allowed by this response contract/,
      field,
    );
  }
  assert.throws(
    () =>
      decodeInstructorPreview(
        {
          ...presentation,
          correctResponse: [{ ...presentation.correctResponse[0], rawKey: "forbidden" }],
        },
        workspace,
      ),
    /allowed by this response contract/,
  );
});

test("author preview represents unavailable source support without fabricating a response", () => {
  const unavailable = { kind: "unavailable", backend: "webwork", reason: "No safe derivation." };
  assert.deepEqual(decodeInstructorPreview(unavailable, workspace), unavailable);
  assert.throws(
    () => decodeInstructorPreview({ ...unavailable, correctResponse: [] }, workspace),
    /allowed by this response contract/,
  );
});

test("the dedicated author client is same-origin, no-store, and never runs until called", async () => {
  const calls = [];
  const client = createInstructorPreviewClient({
    fetch: async (input, init) => {
      calls.push({ input: String(input), init });
      return new Response(JSON.stringify(presentation), {
        headers: { "content-type": "application/json; charset=utf-8", etag: '"4"' },
      });
    },
  });

  assert.equal(calls.length, 0);
  const result = await client.requestPresentation(workspace, 17, '"4"');
  assert.equal(result.kind, "available");
  assert.deepEqual(calls, [
    {
      input: `/api/workspaces/${workspace}/author-preview?seed=17`,
      init: {
        headers: { accept: "application/json", "if-match": '"4"' },
        credentials: "same-origin",
        cache: "no-store",
      },
    },
  ]);
  await assert.rejects(client.requestPresentation(workspace, -1, '"4"'), /whole number/);
  assert.equal(calls.length, 1, "invalid local input must not issue a request");
});

test("the dedicated author client retains no untrusted error response body", async () => {
  const client = createInstructorPreviewClient({
    fetch: async () => new Response('{"key":"do-not-retain"}', { status: 403 }),
  });
  await assert.rejects(
    client.requestPresentation(workspace, 17, '"4"'),
    (error) => error instanceof InstructorPreviewRequestError && error.status === 403,
  );
});

test("the author client requires the exact saved revision in both directions", async () => {
  const mismatch = createInstructorPreviewClient({
    fetch: async () =>
      new Response(JSON.stringify(presentation), {
        headers: { "content-type": "application/json", etag: '"5"' },
      }),
  });
  await assert.rejects(
    mismatch.requestPresentation(workspace, 17, '"4"'),
    /does not match the saved draft/,
  );
  await assert.rejects(
    mismatch.requestPresentation(workspace, 17, '"9223372036854775808"'),
    /strong numeric ETag/,
  );

  const conflict = createInstructorPreviewClient({
    fetch: async () => new Response("", { status: 409 }),
  });
  await assert.rejects(
    conflict.requestPresentation(workspace, 17, '"4"'),
    (error) => error instanceof InstructorPreviewConflictError && error.status === 409,
  );
});
