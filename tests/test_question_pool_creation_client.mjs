import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const FIRST_QUESTION = "7K3M-X9QP";
const SECOND_QUESTION = "2R5X-Z7YA";

function createdPoolResponse(value) {
  return new Response(JSON.stringify(value), {
    status: 201,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
    },
  });
}

test("Question Pool creation sends only attested ordered pins and requires a Revision 1 receipt", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return createdPoolResponse({ questionPoolId: "3S8B-Z4DZ", revisionNumber: 1 });
    },
  });
  const created = await client.createQuestionPool({
    members: [
      { questionId: FIRST_QUESTION, revisionNumber: 3 },
      { questionId: SECOND_QUESTION, revisionNumber: 7 },
    ],
    interchangeabilityAttested: true,
  });

  assert.deepEqual(created, { questionPoolId: "3S8B-Z4DZ", revisionNumber: 1 });
  const request = requests[0];
  assert.ok(request);
  assert.equal(new URL(request.url).pathname, "/api/question-pools");
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("content-type"), "application/json");
  assert.deepEqual(await request.json(), {
    members: [
      { questionId: FIRST_QUESTION, revisionNumber: 3 },
      { questionId: SECOND_QUESTION, revisionNumber: 7 },
    ],
    interchangeabilityAttested: true,
  });

  const malformedReceiptClient = createHttpApiClient({
    fetch: async () => createdPoolResponse({ questionPoolId: "3S8B-Z4DZ", revisionNumber: 2 }),
  });
  await assert.rejects(
    malformedReceiptClient.createQuestionPool({
      members: [{ questionId: FIRST_QUESTION, revisionNumber: 3 }],
      interchangeabilityAttested: true,
    }),
    DecodeError,
  );
});
