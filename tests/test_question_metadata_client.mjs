import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";

const request = {
  publishedQuestionRevisionTuple: {
    publishedQuestionId: "7K3M-79QP",
    revisionNumber: 2,
  },
  expectedMetadataEditNumber: 4,
  metadata: {
    questionTitle: "Amino acid charge",
    questionDescription: "Classify the side chain at the stated pH.",
    questionType: "multipleChoice",
    tags: ["amino acids", "charge"],
    disciplineUuid: "00000000-0000-4000-8000-000000000001",
    subjectUuid: "00000000-0000-4000-8000-000000000002",
    topicUuid: null,
    subtopicUuid: null,
    bloomCognitiveProcess: null,
    bloomKnowledgeDimension: "Factual Knowledge",
  },
};

function jsonResponse(value) {
  return new Response(JSON.stringify(value), {
    headers: { "content-type": "application/json", "cache-control": "no-store" },
  });
}

test("Question metadata save sends an exact Revision and complete bounded replacement", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: async (input, init) => {
      requests.push({ input: String(input), init });
      return jsonResponse({
        publishedQuestionRevisionTuple: request.publishedQuestionRevisionTuple,
        metadataEditNumber: 5,
      });
    },
  });

  const saved = await client.saveQuestionMetadata(request);
  assert.equal(saved.metadataEditNumber, 5);
  assert.deepEqual(saved.publishedQuestionRevisionTuple, request.publishedQuestionRevisionTuple);
  assert.equal(
    new URL(requests[0].input, "https://ple.example").pathname,
    "/ple/api/questions/by-id/7K3M-79QP/metadata",
  );
  assert.deepEqual(JSON.parse(requests[0].init.body), request);
});

test("Question metadata save rejects a receipt for a different Revision", async () => {
  const client = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        publishedQuestionRevisionTuple: {
          publishedQuestionId: "7K3M-79QP",
          revisionNumber: 3,
        },
        metadataEditNumber: 5,
      }),
  });

  await assert.rejects(client.saveQuestionMetadata(request), ApiProtocolError);
});
