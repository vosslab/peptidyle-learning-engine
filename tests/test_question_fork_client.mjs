import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const source = publishedQuestionFixture.publishedQuestion.latestQuestionRevision;
const retryKey = "9f1f2d1f-6d23-4fc2-930f-2bdad8d15fcb";

function createdDraftResponse(value, status = 201) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Question fork client sends only an exact source path and retry key, then accepts one Draft receipt", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return createdDraftResponse({ draftQuestion: "0198e000-0000-7000-8000-000000000001" });
    },
  });

  assert.deepEqual(await client.forkPublishedQuestion(source, retryKey), {
    draftQuestion: "0198e000-0000-7000-8000-000000000001",
  });
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    new URL(request.url).pathname,
    `/api/questions/by-id/${encodeURIComponent(source.questionId)}/revisions/${source.revisionNumber}/fork`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), retryKey);
  assert.equal(request.headers.get("content-type"), null);
  assert.equal(await request.text(), "");
});

test("Question fork client requires a no-store 201 closed Draft receipt", async () => {
  const nonCreatedClient = createHttpApiClient({
    fetch: async () =>
      createdDraftResponse({ draftQuestion: "0198e000-0000-7000-8000-000000000001" }, 200),
  });
  await assert.rejects(nonCreatedClient.forkPublishedQuestion(source, retryKey), ApiProtocolError);

  const cachedReceiptClient = createHttpApiClient({
    fetch: async () =>
      new Response(JSON.stringify({ draftQuestion: "0198e000-0000-7000-8000-000000000001" }), {
        status: 201,
        headers: { "content-type": "application/json" },
      }),
  });
  await assert.rejects(
    cachedReceiptClient.forkPublishedQuestion(source, retryKey),
    ApiProtocolError,
  );

  const malformedReceiptClient = createHttpApiClient({
    fetch: async () =>
      createdDraftResponse({
        draftQuestion: "0198e000-0000-7000-8000-000000000001",
        sourceQuestionId: source.questionId,
      }),
  });
  await assert.rejects(malformedReceiptClient.forkPublishedQuestion(source, retryKey), DecodeError);

  let requests = 0;
  const invalidKeyClient = createHttpApiClient({
    fetch: async () => {
      requests += 1;
      return createdDraftResponse({ draftQuestion: "0198e000-0000-7000-8000-000000000001" });
    },
  });
  await assert.rejects(invalidKeyClient.forkPublishedQuestion(source, "not-a-uuid"), DecodeError);
  assert.equal(requests, 0);
});
