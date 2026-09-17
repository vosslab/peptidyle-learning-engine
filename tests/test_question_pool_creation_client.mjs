import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const FIRST_QUESTION = "7K3M-79QP";
const SECOND_QUESTION = "2R5X-E7YA";

function createdPoolResponse(value) {
  return new Response(JSON.stringify(value), {
    status: 201,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
    },
  });
}

test("Question Pool creation sends explicit metadata and attested ordered pins with a Revision 1 receipt", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return createdPoolResponse({ questionPoolId: "3S8B-24DZ", revisionNumber: 1 });
    },
  });
  const created = await client.createQuestionPool({
    title: "Interchangeable genetics questions",
    description: "Practice reading inheritance patterns.",
    members: [
      { questionId: FIRST_QUESTION, revisionNumber: 3 },
      { questionId: SECOND_QUESTION, revisionNumber: 7 },
    ],
    interchangeabilityAttested: true,
  });

  assert.deepEqual(created, { questionPoolId: "3S8B-24DZ", revisionNumber: 1 });
  const request = requests[0];
  assert.ok(request);
  assert.equal(new URL(request.url).pathname, "/api/question-pools");
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("content-type"), "application/json");
  assert.deepEqual(await request.json(), {
    title: "Interchangeable genetics questions",
    description: "Practice reading inheritance patterns.",
    members: [
      { questionId: FIRST_QUESTION, revisionNumber: 3 },
      { questionId: SECOND_QUESTION, revisionNumber: 7 },
    ],
    interchangeabilityAttested: true,
  });

  const malformedReceiptClient = createHttpApiClient({
    fetch: async () => createdPoolResponse({ questionPoolId: "3S8B-24DZ", revisionNumber: 2 }),
  });
  await assert.rejects(
    malformedReceiptClient.createQuestionPool({
      title: "Interchangeable genetics questions",
      description: "Practice reading inheritance patterns.",
      members: [{ questionId: FIRST_QUESTION, revisionNumber: 3 }],
      interchangeabilityAttested: true,
    }),
    DecodeError,
  );
});

test("Question Pool creation rejects missing or noncanonical metadata before transport", async () => {
  let requests = 0;
  const client = createHttpApiClient({
    fetch: async () => {
      requests += 1;
      return createdPoolResponse({ questionPoolId: "3S8B-24DZ", revisionNumber: 1 });
    },
  });
  const valid = {
    title: "Pool",
    description: "Learning goal",
    members: [{ questionId: FIRST_QUESTION, revisionNumber: 3 }],
    interchangeabilityAttested: true,
  };
  for (const patch of [
    { title: undefined },
    { title: " " },
    { description: "" },
    { title: " Pool " },
    { description: "hidden\u0000control" },
    { title: "a".repeat(513) },
  ]) {
    await assert.rejects(client.createQuestionPool({ ...valid, ...patch }), DecodeError);
  }
  assert.equal(requests, 0);
});
