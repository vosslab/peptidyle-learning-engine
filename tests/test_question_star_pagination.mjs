import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  FIRST_STARRED_QUESTIONS_POSITION,
  starredQuestionsNextPosition,
  starredQuestionsPageSizePosition,
  starredQuestionsPreviousPosition,
} from "../src/pages/starred_questions_model.ts";

const cursor = JSON.stringify({
  pageSize: 50,
  starredAtMicros: 1_790_000_000_000_000,
  publishedQuestionId: "Q123456789",
});

test("Starred Questions carries the selected page size and opaque continuation", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://ple.example"), init);
      requests.push(request);
      return new Response(JSON.stringify({ items: [], nextCursor: null }), {
        status: 200,
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      });
    },
  });

  await client.listStarredQuestions(null, 50);
  await client.listStarredQuestions(cursor, 50);
  const first = new URL(requests[0].url);
  const second = new URL(requests[1].url);
  assert.equal(first.searchParams.get("pageSize"), "50");
  assert.equal(first.searchParams.has("cursor"), false);
  assert.equal(second.searchParams.get("cursor"), cursor);
  assert.equal(second.searchParams.get("pageSize"), "50");
});

test("Starred Questions Previous and page-size changes keep cursor sequences coherent", () => {
  const second = starredQuestionsNextPosition(FIRST_STARRED_QUESTIONS_POSITION, cursor);
  assert.equal(second.inputCursor, cursor);
  assert.deepEqual(starredQuestionsPreviousPosition(second), FIRST_STARRED_QUESTIONS_POSITION);
  assert.deepEqual(starredQuestionsPageSizePosition(100), {
    pageSize: 100,
    inputCursor: null,
    previousCursors: [],
  });
});
