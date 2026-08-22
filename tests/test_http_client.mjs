// Focused strict same-origin HTTP client and decoder behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeCatalogPage,
  decodeDraftQuestionDefinition,
  decodeQuestionEnvelope,
} from "../src/api/decoders.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch, jsonResponse } from "./http_client_test_support.mjs";

test("question decoders reject answer-bearing and provider-secret fields", () => {
  const draft = publishedProblemFixture.draft;
  assert.throws(() => decodeDraftQuestionDefinition({ ...draft, answer: "secret" }), DecodeError);
  assert.throws(
    () =>
      decodeDraftQuestionDefinition({
        ...draft,
        source: { backend: "imathas", provider: "institution", itemRef: "42", token: "secret" },
      }),
    DecodeError,
  );
});

test("issued external-tool envelopes accept only their public marker", () => {
  const envelope = {
    version: "0198e000-0000-7000-8000-000000000004",
    seed: 2,
    title: "External practice item",
    prompt: [],
    response: { kind: "externalTool" },
  };
  assert.deepEqual(decodeQuestionEnvelope(envelope).response, { kind: "externalTool" });
  assert.throws(
    () =>
      decodeQuestionEnvelope({ ...envelope, response: { kind: "externalTool", token: "secret" } }),
    DecodeError,
  );
});

test("catalog pages remain bounded and do not disclose answer material", () => {
  const page = { items: [publishedProblemFixture.catalogProblem], nextCursor: null };
  assert.deepEqual(decodeCatalogPage(page), page);
  assert.throws(() => decodeCatalogPage({ ...page, answerKey: "secret" }), DecodeError);
  assert.throws(
    () => decodeCatalogPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
});

test("prefetch is a body-free same-origin no-store request", async () => {
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://client.example.test"), init);
      requests.push(request);
      return new Response(null, { status: 204 });
    },
  });
  assert.equal(await client.prefetchNextQuestion(attempt.id), null);
  const request = requests[0];
  assert.ok(request);
  assert.equal(request.url, `https://client.example.test/api/attempts/${attempt.id}/prefetch-next`);
  assert.equal(request.method, "POST");
  assert.equal(request.cache, "no-store");
  assert.equal(await request.text(), "");
});

test("prefetch rejects a descriptor with a mismatched issued identity", async () => {
  const predecessor = publishedProblemFixture.attempts[0];
  assert.ok(predecessor);
  const envelope = {
    version: predecessor.questionVersion,
    seed: predecessor.seed,
    presentationNonce: "a".repeat(32),
    title: publishedProblemFixture.publishedProblem.metadata.title,
    prompt: publishedProblemFixture.publishedProblem.prompt,
    response: publishedProblemFixture.publishedProblem.response,
  };
  const client = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        predecessor: predecessor.id,
        run: predecessor.run,
        assignmentPosition: predecessor.assignmentPosition + 1,
        questionVersion: "0198e000-0000-7000-8000-000000000099",
        seed: envelope.seed,
        renderedQuestionSha256: "a".repeat(64),
        envelope,
      }),
  });
  await assert.rejects(client.prefetchNextQuestion(predecessor.id), DecodeError);
});

test("external-tool launch is a strict same-origin route projection", async () => {
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const launchUrl = `/api/attempts/${attempt.id}/external-tool/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(await client.beginExternalToolLaunch(attempt.id), { launchUrl });
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("external-tool launch rejects absolute, foreign, and decorated routes", async () => {
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const routes = [
    "https://client.example.test/api/attempts/x/external-tool/launch",
    "https://foreign.example/api/attempts/x/external-tool/launch",
    "//foreign.example/api/attempts/x/external-tool/launch",
    "/api/attempts/other-attempt/external-tool/launch",
    "/api/health",
    "/api/attempts/x/external-tool/launch?token=secret",
    "/api/attempts/x/external-tool/launch#fragment",
  ];
  for (const launchUrl of routes) {
    const client = createHttpApiClient({
      fetch: async () => jsonResponse({ launchUrl }),
    });
    await assert.rejects(client.beginExternalToolLaunch(attempt.id), DecodeError, launchUrl);
  }
});

test("external-tool submission sends only the marker with its caller idempotency key", async () => {
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const receipt = {
    accepted: true,
    attempt: { ...attempt, response: { kind: "externalTool" }, result: null },
    feedback: null,
    scoringStatus: "current",
    runCompletionStatus: "inProgress",
    nextIssued: null,
    nextPending: false,
  };
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(receipt));
  const client = createHttpApiClient({ fetch: recordingFetch });

  await client.submitResponse(attempt.id, { kind: "externalTool" }, "external-tool-once");
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/attempts/${attempt.id}/external-tool/launch/submission`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "external-tool-once");
  assert.deepEqual(await request.json(), { response: { kind: "externalTool" } });
});
