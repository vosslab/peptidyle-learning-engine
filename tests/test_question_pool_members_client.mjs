import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";

const POOL_ID = "3S8B-24DZ";
const FIRST_QUESTION = "7K3M-79QP";
const SECOND_QUESTION = "2R5X-E7YA";

function noStoreResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
    },
  });
}

test("Pool member save preserves the complete tuple set and its acknowledged Edit Number", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return noStoreResponse({ questionPoolId: POOL_ID, questionPoolEditNumber: 4 });
    },
  });

  const receipt = await client.saveQuestionPoolMembers({
    questionPoolId: POOL_ID,
    expectedQuestionPoolEditNumber: 3,
    members: [
      { publishedQuestionId: FIRST_QUESTION, revisionNumber: 2 },
      { publishedQuestionId: SECOND_QUESTION, revisionNumber: 6 },
    ],
  });

  assert.deepEqual(receipt, { questionPoolId: POOL_ID, questionPoolEditNumber: 4 });
  const request = requests[0];
  assert.ok(request);
  assert.equal(new URL(request.url).pathname, `/api/question-pools/${POOL_ID}/members`);
  assert.equal(request.method, "PUT");
  assert.deepEqual(await request.json(), {
    questionPoolId: POOL_ID,
    expectedQuestionPoolEditNumber: 3,
    members: [
      { publishedQuestionId: FIRST_QUESTION, revisionNumber: 2 },
      { publishedQuestionId: SECOND_QUESTION, revisionNumber: 6 },
    ],
  });
});

test("Pool member save rejects duplicate Question IDs and invalid receipts", async () => {
  let requests = 0;
  const client = createHttpApiClient({
    fetch: async () => {
      requests += 1;
      return noStoreResponse({
        questionPoolId: POOL_ID,
        questionPoolEditNumber: 4,
        unexpected: true,
      });
    },
  });
  const valid = {
    questionPoolId: POOL_ID,
    expectedQuestionPoolEditNumber: 3,
    members: [{ publishedQuestionId: FIRST_QUESTION, revisionNumber: 2 }],
  };

  await assert.rejects(
    client.saveQuestionPoolMembers({
      ...valid,
      members: [...valid.members, { publishedQuestionId: FIRST_QUESTION, revisionNumber: 3 }],
    }),
    DecodeError,
  );
  assert.equal(requests, 0);
  await assert.rejects(client.saveQuestionPoolMembers(valid), DecodeError);
  assert.equal(requests, 1);
});

test("Pool member save surfaces stale Edit Number failures without decoding an error body", async () => {
  const client = createHttpApiClient({
    fetch: async () => noStoreResponse({ error: "Pool membership changed" }, 412),
  });
  await assert.rejects(
    client.saveQuestionPoolMembers({
      questionPoolId: POOL_ID,
      expectedQuestionPoolEditNumber: 3,
      members: [{ publishedQuestionId: FIRST_QUESTION, revisionNumber: 2 }],
    }),
    (error) => error instanceof ApiRequestError && error.status === 412,
  );
});
