// WP-C7 behavior tests for the server-free mock API.

import assert from "node:assert/strict";
import test from "node:test";

import {
  createMockFetch,
  mockApiHandlers,
  MOCK_ROUTE_GROUPS,
  prefetchFixtureAttempt,
  prefetchedFixtureAttempt,
} from "../src/api/mock/handlers.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";

test("mock handlers cover every planned API route group exactly once", () => {
  const actual = mockApiHandlers.map((handler) => handler.group).toSorted();
  const expected = [...MOCK_ROUTE_GROUPS].toSorted();
  assert.deepEqual(actual, expected);
});

test("all route groups answer without a server", async () => {
  const mockFetch = createMockFetch();
  const probes = [
    { group: "auth", path: "/api/auth/session", method: "GET" },
    { group: "catalog", path: "/api/problems", method: "GET" },
    { group: "course", path: "/api/courses", method: "GET" },
    { group: "run", path: "/api/runs", method: "POST" },
    {
      group: "asset",
      path: "/api/assets/0198e000-0000-7000-8000-000000000010",
      method: "GET",
    },
  ];

  for (const probe of probes) {
    const response = await mockFetch(probe.path, { method: probe.method });
    assert.equal(response.ok, true, `${probe.group} returned ${response.status}`);
  }
});

test("gradebook mock serves compact summary rows through the course route", async () => {
  const mockFetch = createMockFetch();
  const response = await mockFetch(
    "/api/courses/0198e000-0000-7000-8000-000000000014/gradebook?cursor=next&pageSize=25",
  );
  assert.equal(response.status, 200);
  const payload = await response.json();
  assert.deepEqual(Object.keys(payload).toSorted(), ["items", "nextCursor"]);
  assert.equal(payload.nextCursor, null);
  assert.equal(payload.items.length, 1);
  assert.deepEqual(Object.keys(payload.items[0]).toSorted(), [
    "assignmentId",
    "assignmentTitle",
    "courseId",
    "enrollmentId",
    "studentId",
    "summary",
    "tenant",
  ]);
  assert.equal(payload.items[0].summary.tenant, payload.items[0].tenant);
  assert.equal(payload.items[0].summary.enrollment, payload.items[0].enrollmentId);
  assert.equal("runs" in payload.items[0], false);
  assert.equal("attempts" in payload.items[0], false);
});

test("mock history preserves fresh seeds and full attempt provenance", async () => {
  const mockFetch = createMockFetch();
  const response = await mockFetch("/api/runs/0198e000-0000-7000-8000-000000000023/attempts");
  assert.equal(response.status, 200);

  const payload = await response.json();
  assert.equal(typeof payload, "object");
  assert.notEqual(payload, null);
  assert.ok("items" in payload);
  assert.ok(Array.isArray(payload.items));
  assert.equal(payload.items.length, 1);

  const attempt = payload.items[0];
  assert.equal(typeof attempt, "object");
  assert.notEqual(attempt, null);
  assert.ok("seed" in attempt);
  assert.ok("parameterHash" in attempt);
  assert.ok("provenance" in attempt);
  assert.equal(typeof attempt.seed, "number");
  assert.equal(typeof attempt.parameterHash, "string");
  assert.equal(attempt.parameterHash.length, 64);
});

test("issued-question route returns a generated key-free variant bound to its attempt", async () => {
  const mockFetch = createMockFetch();
  const attemptResponse = await mockFetch(
    "/api/runs/0198e000-0000-7000-8000-000000000023/attempts",
  );
  const attempts = await attemptResponse.json();
  const attempt = attempts.items[0];
  const response = await mockFetch(`/api/attempts/${attempt.id}/question`);
  const envelope = await response.json();
  assert.equal(response.status, 200);
  assert.equal(envelope.version, attempt.questionVersion);
  assert.equal(envelope.seed, attempt.seed);
  assert.equal(envelope.title, "Peptide bond resonance and planarity");
  assert.equal("grading" in envelope, false);
  assert.equal("answer" in envelope, false);
  assert.ok(envelope.prompt[0].markdown.includes("proline"));
});

test("mock prefetch has the same no-successor cache-miss semantics as its typed client", async () => {
  const mockFetch = createMockFetch();
  const client = createMockApiClient();
  const attemptId = "0198e000-0000-7000-8000-000000000030";

  const response = await mockFetch(`/api/attempts/${attemptId}/prefetch-next`, { method: "POST" });
  assert.equal(response.status, 204, "the one-question fixture has no successor to reserve");
  assert.equal(await response.text(), "");
  assert.equal(await client.prefetchNextQuestion(attemptId), null);
});

test("two-position mock prefetch is key-free, body-free, and bound to its receipt", async () => {
  const mockFetch = createMockFetch();
  const client = createMockApiClient();
  const first = prefetchFixtureAttempt.id;
  const second = prefetchedFixtureAttempt.id;
  const prefetched = await client.prefetchNextQuestion(first);
  assert.notEqual(prefetched, null);
  assert.equal(prefetched.predecessor, first);
  assert.equal(prefetched.run, prefetchFixtureAttempt.run);
  assert.equal(prefetched.envelope.version, prefetched.questionVersion);
  assert.equal(prefetched.envelope.seed, prefetched.seed);
  assert.doesNotMatch(JSON.stringify(prefetched), /key|provider|source|provenance/i);

  const receipt = await client.submitResponse(
    first,
    { kind: "multipleChoice", selected: ["carbonyl"] },
    "two-position-key",
  );
  assert.deepEqual(receipt.nextIssued, {
    id: second,
    run: prefetched.run,
    questionVersion: prefetched.questionVersion,
    seed: prefetched.seed,
    deadline: prefetchedFixtureAttempt.timer.deadline,
    assignmentPosition: prefetched.assignmentPosition,
    renderedQuestionSha256: prefetched.renderedQuestionSha256,
  });

  const malformed = await mockFetch(`/api/attempts/${first}/prefetch-next`, {
    method: "POST",
    body: "{}",
  });
  assert.equal(malformed.status, 400);
  const wrongPath = await mockFetch(`/api/attempts/${first}/prefetch-next/forged`, {
    method: "POST",
  });
  assert.equal(wrongPath.status, 404);
  const exhausted = await mockFetch(`/api/attempts/${second}/prefetch-next`, { method: "POST" });
  assert.equal(exhausted.status, 204);
});

test("mock client decodes handler prefetch responses instead of trusting fixture expectations", async () => {
  const handler = createMockFetch();
  const client = createMockApiClient({
    fetch: async (input, init) => {
      const response = await handler(input, init);
      if (!String(input).endsWith(`/api/attempts/${prefetchFixtureAttempt.id}/prefetch-next`)) {
        return response;
      }
      const value = await response.json();
      value.provider = "forged";
      return new Response(JSON.stringify(value), {
        headers: { "content-type": "application/json" },
      });
    },
  });
  await assert.rejects(client.prefetchNextQuestion(prefetchFixtureAttempt.id));
});

test("external-tool launch mock projects only an inert same-origin broker path", async () => {
  const mockFetch = createMockFetch();
  const attemptId = "0198e000-0000-7000-8000-000000000034";
  const response = await mockFetch(`/api/attempts/${attemptId}/external-tool-launch`);
  assert.equal(response.status, 200);
  const projection = await response.json();
  assert.deepEqual(Object.keys(projection), ["launchUrl"]);
  assert.equal(projection.launchUrl, `/api/attempts/${attemptId}/external-tool/launch`);
  assert.ok(!JSON.stringify(projection).match(/provider|token|score|answer|https?:/i));
});

test("external-tool launch mock requires its external capability and has no broker fallback", async () => {
  const mockFetch = createMockFetch();
  const nonExternalId = "0198e000-0000-7000-8000-000000000030";
  const externalId = "0198e000-0000-7000-8000-000000000034";

  const nonExternal = await mockFetch(`/api/attempts/${nonExternalId}/external-tool-launch`);
  assert.equal(nonExternal.status, 404);
  assert.ok(!JSON.stringify(await nonExternal.json()).includes("launchUrl"));

  const copiedBroker = await mockFetch(`/api/attempts/${externalId}/external-tool/launch`);
  assert.equal(copiedBroker.status, 404);
  const body = await copiedBroker.json();
  assert.equal("id" in body, false, "broker path must not fall through to attempt lookup");
  assert.equal("launchUrl" in body, false);
});

test("external-tool mock submission is marker-only and scoped to its capable fixture", async () => {
  const mockFetch = createMockFetch();
  const externalId = "0198e000-0000-7000-8000-000000000034";
  const external = await mockFetch(`/api/attempts/${externalId}/external-tool/launch/submission`, {
    method: "POST",
    headers: { "content-type": "application/json", "idempotency-key": "mock-external-key" },
    body: JSON.stringify({ response: { kind: "externalTool" } }),
  });
  assert.equal(external.status, 200);
  const receipt = await external.json();
  assert.equal(receipt.attempt.response.kind, "externalTool");
  assert.equal(receipt.attempt.result, null);
  assert.equal(receipt.feedback, null);
  assert.ok(!JSON.stringify(receipt).match(/provider|token|score|answer|solution/i));

  const ordinary = await mockFetch(
    "/api/attempts/0198e000-0000-7000-8000-000000000030/external-tool/launch/submission",
    { method: "POST" },
  );
  assert.equal(ordinary.status, 404);
});

test("external-tool mock child handler rejects malformed keys and every non-marker body", async () => {
  const mockFetch = createMockFetch();
  const path = "/api/attempts/0198e000-0000-7000-8000-000000000034/external-tool/launch/submission";
  const marker = JSON.stringify({ response: { kind: "externalTool" } });

  const missingKey = await mockFetch(path, { method: "POST", body: marker });
  assert.equal(missingKey.status, 400);
  const malformedKey = await mockFetch(path, {
    method: "POST",
    headers: { "idempotency-key": "bad key" },
    body: marker,
  });
  assert.equal(malformedKey.status, 400);
  // Fetch normalizes terminal header whitespace before the mock sees it. Call
  // the handler with a transport-shaped request to prove its own validation
  // still rejects a raw control character, matching the server parser.
  const runHandler = mockApiHandlers.find((handler) => handler.group === "run");
  assert.notEqual(runHandler, undefined);
  const trailingNewlineKey = await runHandler.respond({
    method: "POST",
    url: `https://mock.peptidyle.invalid${path}`,
    headers: { get: () => "strict-mock-key\n" },
    text: async () => marker,
  });
  assert.equal(trailingNewlineKey.status, 400);

  const invalidBodies = [
    "{",
    JSON.stringify({}),
    JSON.stringify({ response: { kind: "externalTool" }, provider: "forged" }),
    JSON.stringify({ response: { kind: "externalTool", score: 1 } }),
    JSON.stringify({ response: { kind: "externalTool", token: "forged" } }),
    JSON.stringify({ response: { kind: "multipleChoice", selected: ["carbonyl"] } }),
    '{"response":{"kind":"numeric"},"response":{"kind":"externalTool"}}',
    '{"response":{"kind":"externalTool","kind":"externalTool"}}',
  ];
  for (const body of invalidBodies) {
    const response = await mockFetch(path, {
      method: "POST",
      headers: { "idempotency-key": "strict-mock-key" },
      body,
    });
    assert.equal(response.status, 400, `body must be rejected: ${body}`);
  }

  const ordinaryAttempt = await mockFetch(
    "/api/attempts/0198e000-0000-7000-8000-000000000030/external-tool/launch/submission",
    {
      method: "POST",
      headers: { "idempotency-key": "strict-mock-key" },
      body: marker,
    },
  );
  assert.equal(ordinaryAttempt.status, 404);
});

test("issued-question mock projects only the rendered envelope fields", async () => {
  const mockFetch = createMockFetch();
  const response = await mockFetch("/api/attempts/0198e000-0000-7000-8000-000000000030/question");
  const envelope = await response.json();
  assert.deepEqual(Object.keys(envelope).toSorted(), [
    "prompt",
    "response",
    "seed",
    "title",
    "version",
  ]);
  assert.deepEqual(Object.keys(envelope.prompt[0]).toSorted(), ["kind", "markdown"]);
  assert.deepEqual(Object.keys(envelope.response).toSorted(), ["choices", "kind", "selection"]);
  assert.deepEqual(Object.keys(envelope.response.choices[0]).toSorted(), ["body", "id"]);
  assert.deepEqual(Object.keys(envelope.response.choices[0].body[0]).toSorted(), [
    "kind",
    "markdown",
  ]);
  assert.deepEqual(Object.keys(envelope.response.selection).toSorted(), ["kind"]);
});

test("submission fixtures model disclosure at the server boundary without private fields", async () => {
  const mockFetch = createMockFetch();
  const cases = [
    ["0198e000-0000-7000-8000-000000000030", ["correctness", "hint"]],
    [
      "0198e000-0000-7000-8000-000000000031",
      ["correctResponse", "correctness", "hint", "pointsEarned", "pointsPossible", "rationale"],
    ],
    ["0198e000-0000-7000-8000-000000000032", null],
    ["0198e000-0000-7000-8000-000000000033", null],
    ["0198e000-0000-7000-8000-000000000034", null],
  ];
  for (const [attemptId, expectedFields] of cases) {
    const response = await mockFetch(`/api/submissions/${attemptId}`, { method: "POST" });
    assert.equal(response.status, 200);
    const raw = await response.text();
    assert.doesNotMatch(
      raw,
      /"(?:answerKey|key|provider|token|source|solution|checker|rubric)"\s*:/i,
    );
    const receipt = JSON.parse(raw);
    assert.equal(
      receipt.feedback === null ? null : Object.keys(receipt.feedback).toSorted().join(","),
      expectedFields === null ? null : [...expectedFields].toSorted().join(","),
    );
  }
});

test("run summary stays bounded, cursor-paged, and contains only server-redacted outcomes", async () => {
  const mockFetch = createMockFetch();
  const runId = "0198e000-0000-7000-8000-000000000023";
  const first = await mockFetch(`/api/runs/${runId}/summary?pageSize=30`);
  assert.equal(first.status, 200);
  const firstPayload = await first.json();
  assert.equal(firstPayload.outcomes.items.length, 30);
  assert.equal(firstPayload.outcomes.nextCursor, "summary:30");
  assert.doesNotMatch(
    JSON.stringify(firstPayload),
    /"(?:answerKey|checker|provider|provenance|source|launchUrl|feedbackContent)"\s*:/,
  );
  assert.deepEqual(Object.keys(firstPayload.outcomes.items[0]).toSorted(), [
    "assignmentPosition",
    "attempt",
    "feedback",
    "response",
    "submittedAt",
  ]);
  const next = await mockFetch(`/api/runs/${runId}/summary?pageSize=30&cursor=summary%3A30`);
  const nextPayload = await next.json();
  assert.equal(nextPayload.outcomes.items.length, 1);
  assert.equal(nextPayload.outcomes.nextCursor, null);
});

test("run summary mock and client reject malformed cursor and page bounds", async () => {
  const mockFetch = createMockFetch();
  const runId = "0198e000-0000-7000-8000-000000000023";
  for (const query of [
    "cursor=",
    `cursor=${"x".repeat(513)}`,
    "cursor=summary%3A30junk",
    "pageSize=0",
    "pageSize=101",
  ]) {
    const response = await mockFetch(`/api/runs/${runId}/summary?${query}`);
    assert.equal(response.status, 404, query);
  }
  const client = createMockApiClient();
  await assert.rejects(() => client.getRunSummary(runId, "", 30));
  await assert.rejects(() => client.getRunSummary(runId, "x".repeat(513), 30));
  await assert.rejects(() => client.getRunSummary(runId, "summary:30junk", 30));
  await assert.rejects(() => client.getRunSummary(runId, undefined, 0));
  await assert.rejects(() => client.getRunSummary(runId, undefined, 101));
  await client.getRunSummary(runId, "summary:30", 30);
});

test("unknown paths do not fall through to the real network", async () => {
  const response = await createMockFetch()("/api/not-a-route");
  assert.equal(response.status, 404);
});
