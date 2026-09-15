import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const question = publishedQuestionFixture.publishedQuestion;

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Question Watch client uses one closed self-only endpoint", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return noStoreJson({ watching: request.method === "PUT" });
    },
  });

  assert.deepEqual(await client.getQuestionWatch(question.questionId), { watching: false });
  assert.deepEqual(await client.setQuestionWatch(question.questionId, true), { watching: true });
  assert.equal(
    new URL(requests[0].url).pathname,
    `/api/questions/by-id/${encodeURIComponent(question.questionId)}/stewardship/watch`,
  );
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[1].method, "PUT");
  assert.deepEqual(await requests[1].json(), { watching: true });
});

test("Question Watch client rejects any watcher disclosure beyond its boolean", async () => {
  const client = createHttpApiClient({
    fetch: async () => noStoreJson({ watching: true, watcherCount: 1 }),
  });
  await assert.rejects(client.getQuestionWatch(question.questionId), DecodeError);
});
