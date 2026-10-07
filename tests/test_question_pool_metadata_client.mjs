import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { DecodeError } from "../src/api/decoder.ts";

const questionPoolId = "3S8B-24DZ";
const current = {
  questionPoolId,
  questionPoolMetadataEditNumber: 7,
  title: "Protein structure problems",
  description: "Use the supplied structure and data.",
  topicUuid: null,
  subtopicUuid: null,
  tags: ["protein", "structure"],
  bloomCognitiveProcess: "Analyze",
  bloomKnowledgeDimension: null,
};

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store" },
  });
}

test("Question Pool metadata GET reads the current independent replacement and counter", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: async (input, init) => {
      requests.push({ input: String(input), init });
      return jsonResponse(current);
    },
  });

  assert.deepEqual(await client.getCurrentQuestionPoolMetadata(questionPoolId), current);
  assert.equal(
    new URL(requests[0].input, "https://ple.example").pathname,
    `/ple/api/question-pools/${questionPoolId}/metadata`,
  );
  assert.equal(requests[0].init.method, "GET");
  assert.equal(requests[0].init.body, undefined);
});

test("Question Pool metadata PUT carries one full nullable replacement and exact counter", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push({ input: String(input), init });
      return jsonResponse({ questionPoolId, questionPoolMetadataEditNumber: 8 });
    },
  });
  const request = {
    questionPoolId,
    expectedMetadataEditNumber: 7,
    metadata: {
      title: current.title,
      description: current.description,
      topicUuid: null,
      subtopicUuid: null,
      tags: current.tags,
      bloomCognitiveProcess: "Create",
      bloomKnowledgeDimension: null,
    },
  };

  assert.deepEqual(await client.saveQuestionPoolMetadata(request), {
    questionPoolId,
    questionPoolMetadataEditNumber: 8,
  });
  assert.equal(
    new URL(requests[0].input, "https://ple.example").pathname,
    `/api/question-pools/${questionPoolId}/metadata`,
  );
  assert.equal(requests[0].init.method, "PUT");
  assert.deepEqual(JSON.parse(requests[0].init.body), request);
});

test("Question Pool metadata client rejects mismatched targets, malformed reads, and stale writes", async () => {
  const wrongTargetClient = createHttpApiClient({
    fetch: async () => jsonResponse({ ...current, questionPoolId: "7K3M-79QP" }),
  });
  await assert.rejects(
    wrongTargetClient.getCurrentQuestionPoolMetadata(questionPoolId),
    ApiProtocolError,
  );

  const malformedClient = createHttpApiClient({
    fetch: async () => jsonResponse({ ...current, surprise: true }),
  });
  await assert.rejects(malformedClient.getCurrentQuestionPoolMetadata(questionPoolId), DecodeError);

  const staleClient = createHttpApiClient({ fetch: async () => jsonResponse({}, 412) });
  await assert.rejects(
    staleClient.saveQuestionPoolMetadata({
      questionPoolId,
      expectedMetadataEditNumber: 7,
      metadata: {
        title: current.title,
        description: current.description,
        topicUuid: null,
        subtopicUuid: null,
        tags: current.tags,
        bloomCognitiveProcess: null,
        bloomKnowledgeDimension: null,
      },
    }),
    ApiRequestError,
  );
});
