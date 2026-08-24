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
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
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
  assert.equal(await client.prefetchNextQuestion(course.id, assignment.id, attempt.id), null);
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/prefetch-next`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.cache, "no-store");
  assert.equal(await request.text(), "");
});

test("run start uses the explicit nested course and assignment route without a body", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const run = publishedProblemFixture.runs[0];
  assert.ok(run);
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(run));
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(await client.startRun(course.id, assignment.id), run);
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/runs`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("content-type"), null);
  assert.equal(await request.text(), "");
});

test("prefetch rejects a descriptor with a mismatched issued identity", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
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
  await assert.rejects(
    client.prefetchNextQuestion(course.id, assignment.id, predecessor.id),
    DecodeError,
  );
});

test("external-tool launch is a strict same-origin route projection", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const launchUrl = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(await client.beginExternalToolLaunch(course.id, assignment.id, attempt.id), {
    launchUrl,
  });
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("external-tool launch rejects absolute, foreign, and decorated routes", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const expected = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch`;
  const routes = [
    `https://client.example.test${expected}`,
    `https://foreign.example${expected}`,
    `//foreign.example${expected}`,
    expected.replace(course.id, "other-course"),
    expected.replace(assignment.id, "other-assignment"),
    expected.replace(attempt.id, "other-attempt"),
    "/api/health",
    `${expected}?token=secret`,
    `${expected}#fragment`,
  ];
  for (const launchUrl of routes) {
    const client = createHttpApiClient({
      fetch: async () => jsonResponse({ launchUrl }),
    });
    await assert.rejects(
      client.beginExternalToolLaunch(course.id, assignment.id, attempt.id),
      DecodeError,
      launchUrl,
    );
  }
});

test("ordinary submission uses the explicit nested binding and answer-only body", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const response = { kind: "numeric", value: 18 };
  const receipt = {
    accepted: true,
    attempt: { ...attempt, response, result: null },
    feedback: null,
    scoringStatus: "current",
    runCompletionStatus: "inProgress",
    nextIssued: null,
    nextPending: false,
  };
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(receipt));
  const client = createHttpApiClient({ fetch: recordingFetch });

  await client.submitResponse(course.id, assignment.id, attempt.id, response, "nested-once");
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/submissions`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "nested-once");
  assert.deepEqual(await request.json(), { response });
});

test("external-tool submission sends only the marker with its caller idempotency key", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
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

  await client.submitResponse(
    course.id,
    assignment.id,
    attempt.id,
    { kind: "externalTool" },
    "external-tool-once",
  );
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch/submission`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "external-tool-once");
  assert.deepEqual(await request.json(), { response: { kind: "externalTool" } });
});
