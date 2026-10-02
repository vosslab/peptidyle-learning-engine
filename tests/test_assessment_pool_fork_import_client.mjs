import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";

test("Assessment Pool import sends the inspected source Pool Edit Number", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return new Response(
        JSON.stringify({
          assessmentEntryId: "0198e000-0000-7000-8000-000000000018",
          questionPoolId: "7K3M-79QP",
          questionPoolEditNumber: 1,
          assessmentEditNumber: "2",
        }),
        {
          status: 201,
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json",
            etag: '"2"',
          },
        },
      );
    },
  });

  await client.importAssessmentQuestionPoolFork(
    "CI6F2R8TA0",
    "A7K3M2QXF",
    {
      sourceQuestionPoolId: "7K3M-79QP",
      expectedSourceQuestionPoolEditNumber: 9,
      authoredPosition: 0,
      selectionCount: 1,
      pointsPerItem: 1,
      selectedQuestionOrder: "questionPoolOrder",
      scoringRule: "normal",
    },
    "1",
  );

  const request = requests[0];
  assert.ok(request);
  assert.equal(request.headers.get("if-match"), '"1"');
  assert.deepEqual(await request.json(), {
    sourceQuestionPoolId: "7K3M-79QP",
    expectedSourceQuestionPoolEditNumber: 9,
    authoredPosition: 0,
    selectionCount: 1,
    pointsPerItem: 1,
    selectedQuestionOrder: "questionPoolOrder",
    scoringRule: "normal",
  });
});

test("Assessment Pool import rejects a missing source Pool Edit Number before transport", async () => {
  let requests = 0;
  const client = createHttpApiClient({
    fetch: async () => {
      requests += 1;
      throw new Error("unexpected request");
    },
  });

  await assert.rejects(
    client.importAssessmentQuestionPoolFork(
      "CI6F2R8TA0",
      "A7K3M2QXF",
      {
        sourceQuestionPoolId: "7K3M-79QP",
        expectedSourceQuestionPoolEditNumber: 0,
        authoredPosition: 0,
        selectionCount: 1,
        pointsPerItem: 1,
        selectedQuestionOrder: "questionPoolOrder",
        scoringRule: "normal",
      },
      "1",
    ),
    ApiProtocolError,
  );
  assert.equal(requests, 0);
});
