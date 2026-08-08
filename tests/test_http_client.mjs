// MOD-CLIENT behavior tests for the strict same-origin HTTP transport.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { ApiProtocolError, ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { createMockFetch } from "../src/api/mock/handlers.ts";

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

function createFixtureFetch() {
  const mockFetch = createMockFetch();
  const requests = [];

  async function fixtureFetch(input, init) {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    requests.push(request.clone());
    const url = new URL(request.url);
    const path = url.pathname.replace(/^\/ple/, "");
    if (path === "/api/validation/response-format") {
      return jsonResponse({ violations: [] });
    }
    if (path === "/api/validation/timer") {
      return jsonResponse("open");
    }
    if (path === "/api/validation/assignment-capabilities") {
      return jsonResponse([]);
    }
    const body = request.method === "GET" ? undefined : await request.text();
    return mockFetch(`${path}${url.search}`, {
      method: request.method,
      headers: request.headers,
      body,
    });
  }

  return { fixtureFetch, requests };
}

test("the HTTP client decodes every implemented route and composes a run screen", async () => {
  const { fixtureFetch, requests } = createFixtureFetch();
  const client = createHttpApiClient({ fetch: fixtureFetch, basePath: "/ple/" });
  const fixture = publishedProblemFixture;

  assert.deepEqual(await client.getSession(), {
    authenticated: true,
    tenant: fixture.enrollment.tenant,
    user: {
      id: fixture.enrollment.user,
      displayName: "Fixture Student",
      roles: ["student"],
    },
  });
  assert.equal(
    (await client.listProblems("next page")).items[0].problem,
    fixture.catalogProblem.problem,
  );
  assert.equal(
    (
      await client.getProblemVersion(
        fixture.publishedProblem.problem,
        fixture.publishedProblem.version,
      )
    ).version,
    fixture.publishedProblem.version,
  );
  assert.deepEqual((await client.listTaxonomy()).items, fixture.publishedProblem.metadata.taxonomy);
  assert.equal((await client.listCourses()).items[0].id, fixture.course.id);
  assert.equal((await client.getCourse(fixture.course.id)).role, "student");
  assert.equal(
    (await client.listAssignments(fixture.course.id)).items[0].id,
    fixture.assignment.id,
  );
  assert.equal((await client.getAssignment(fixture.assignment.id)).courseId, fixture.course.id);
  assert.equal(
    (await client.getEnrollment(fixture.enrollment.id)).summary.enrollment,
    fixture.enrollment.id,
  );
  assert.equal((await client.listRuns(fixture.enrollment.id)).items.length, fixture.runs.length);

  const activeRun = await client.startRun(fixture.assignment.id);
  assert.equal((await client.getRun(activeRun.id)).id, activeRun.id);
  const attempts = await client.listAttempts(activeRun.id);
  assert.equal((await client.getAttempt(attempts.items[0].id)).run, activeRun.id);
  assert.equal(
    (
      await client.submitResponse(
        attempts.items[0].id,
        { kind: "multipleChoice", selected: ["carbonyl"] },
        "stable-retry-key",
      )
    ).accepted,
    true,
  );
  assert.equal((await client.getSummary(fixture.enrollment.id)).enrollment, fixture.enrollment.id);
  const screen = await client.getRunScreen(activeRun.id);
  assert.equal(screen.course.id, fixture.course.id);
  assert.equal(screen.assignment.id, fixture.assignment.id);
  assert.equal(screen.attempt.run, activeRun.id);
  assert.equal(screen.question.version, screen.attempt.questionVersion);

  assert.deepEqual(
    await client.validateResponseFormatOnServer(fixture.publishedProblem.response, {
      kind: "multipleChoice",
      selected: ["carbonyl"],
    }),
    { violations: [] },
  );
  assert.equal(
    await client.timerVerdictOnServer({
      policy: { kind: "perQuestion", seconds: 30, graceSeconds: 2 },
      timer: { issuedAt: 1_000, deadline: 31_000, submittedAt: null },
      evaluatedAt: 2_000,
      pauseExtensionMillis: 0,
    }),
    "open",
  );
  assert.deepEqual(
    await client.validateAssignmentConfigOnServer({
      questions: [{ question: fixture.publishedProblem, backendCapabilities: [] }],
      requiredCapabilities: [],
    }),
    [],
  );
  assert.equal(client.assetUrl(fixture.assets[0].id), `/ple/api/assets/${fixture.assets[0].id}`);

  assert.ok(requests.every((request) => request.credentials === "same-origin"));
  assert.ok(requests.every((request) => request.cache === "no-store"));
  assert.ok(requests.some((request) => request.url.endsWith("?cursor=next+page")));
  const submission = requests.find((request) => request.url.includes("/api/submissions/"));
  assert.notEqual(submission, undefined);
  assert.equal(submission.headers.get("idempotency-key"), "stable-retry-key");
  assert.equal(submission.headers.get("content-type"), "application/json");
});

test("the HTTP boundary rejects malformed success bodies without a cast", async () => {
  const malformed = structuredClone(publishedProblemFixture.assignment);
  malformed.courseId = "not-a-uuid";
  const client = createHttpApiClient({
    fetch: () => Promise.resolve(jsonResponse(malformed)),
  });

  await assert.rejects(
    client.getAssignment(publishedProblemFixture.assignment.id),
    (error) => error instanceof DecodeError && error.message === "response.courseId must be a UUID",
  );
});

test("HTTP and protocol failures are distinct and do not echo response bodies", async () => {
  const rejected = createHttpApiClient({
    fetch: () => Promise.resolve(jsonResponse({ error: "private database detail" }, 503)),
  });
  await assert.rejects(
    rejected.getSession(),
    (error) =>
      error instanceof ApiRequestError &&
      error.status === 503 &&
      !error.message.includes("private database detail"),
  );

  const nonJson = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response("<html>proxy error</html>", {
          headers: { "content-type": "text/html" },
        }),
      ),
  });
  await assert.rejects(nonJson.getSession(), ApiProtocolError);
});

test("the configured API prefix cannot select another origin", () => {
  for (const basePath of ["https://evil.example", "//evil.example", "/api?token=x", "/api#x"]) {
    assert.throws(() => createHttpApiClient({ basePath }), /same-origin path/);
  }
});

test("run-screen composition rejects inconsistent resource relationships", async () => {
  const { fixtureFetch } = createFixtureFetch();
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const response = await fixtureFetch(input, init);
      if (input.toString().includes("/api/enrollments/")) {
        const value = await response.json();
        value.summary.enrollment = publishedProblemFixture.course.id;
        return jsonResponse(value);
      }
      return response;
    },
  });

  await assert.rejects(
    client.getRunScreen(publishedProblemFixture.runs.at(-1).id),
    (error) =>
      error instanceof ApiProtocolError &&
      error.message === "Run screen enrollment records are inconsistent",
  );
});
