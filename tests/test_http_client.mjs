// MOD-CLIENT behavior tests for the strict same-origin HTTP transport.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeDraftQuestionDefinition,
  decodeExternalToolLaunch,
  decodeCatalogPage,
  decodeGradebookPage,
  decodeQuestionAttempt,
  decodeQuestionDefinition,
  decodeQuestionEnvelope,
  decodeIssuedPresentationEnvelope,
  decodeStudentResponse,
} from "../src/api/decoders.ts";
import { ApiProtocolError, ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { createHttpLocalCredentialLogin } from "../src/api/http_client/local_development_auth.ts";
import { createMockFetch, issuedQuestionWireForAttempt } from "../src/api/mock/handlers.ts";
import { validateResponseFormatInMock } from "../src/api/mock/format_validation.ts";
import { createFixtureFetch, jsonResponse } from "./http_client_test_support.mjs";

test("question decoders reject published, grading, and provider-secret fields instead of dropping them", () => {
  const draft = publishedProblemFixture.draft;
  assert.deepEqual(decodeDraftQuestionDefinition(draft).source, draft.source);

  for (const forbidden of [
    "problem",
    "version",
    "answer",
    "answerKey",
    "sourceArtifact",
    "launch",
  ]) {
    assert.throws(
      () => decodeDraftQuestionDefinition({ ...draft, [forbidden]: "server-only" }),
      DecodeError,
      `draft must reject ${forbidden}`,
    );
  }
  assert.throws(
    () =>
      decodeDraftQuestionDefinition({
        ...draft,
        source: { backend: "imathas", provider: "institution", itemRef: "42", token: "secret" },
      }),
    DecodeError,
    "draft source must reject a provider secret",
  );
  assert.throws(
    () =>
      decodeQuestionDefinition({ ...publishedProblemFixture.publishedProblem, answer: "secret" }),
    DecodeError,
    "published definition must reject an answer field",
  );
});

test("external-tool response markers are exact and cannot carry browser provider material", () => {
  const envelope = {
    version: "0198e000-0000-7000-8000-000000000004",
    seed: 2,
    title: "External practice item",
    prompt: [],
    response: { kind: "externalTool" },
  };
  assert.deepEqual(decodeQuestionEnvelope(envelope).response, { kind: "externalTool" });

  for (const invalidTitle of ["", " \t\n", "x".repeat(513), "🧬".repeat(513)]) {
    assert.throws(
      () => decodeQuestionEnvelope({ ...envelope, title: invalidTitle }),
      DecodeError,
      "issued titles must be present and bounded",
    );
  }

  for (const forbidden of ["score", "correct", "result", "provider", "token", "launchUrl"]) {
    assert.throws(
      () =>
        decodeQuestionEnvelope({
          ...envelope,
          response: { kind: "externalTool", [forbidden]: true },
        }),
      DecodeError,
      `external response definition must reject ${forbidden}`,
    );
  }

  const attempt = structuredClone(publishedProblemFixture.attempts[0]);
  assert.equal(decodeQuestionAttempt(attempt).issuedCapability, "flatPresentation");
  delete attempt.issuedCapability;
  assert.throws(
    () => decodeQuestionAttempt(attempt),
    DecodeError,
    "attempts must retain their immutable issued capability",
  );
  attempt.issuedCapability = "flatPresentation";
  attempt.issuedCapability = "legacyFallback";
  assert.throws(
    () => decodeQuestionAttempt(attempt),
    DecodeError,
    "attempt capabilities must reject unknown or legacy recovery values",
  );
  attempt.issuedCapability = "flatPresentation";
  attempt.response = { kind: "externalTool" };
  assert.deepEqual(decodeQuestionAttempt(attempt).response, { kind: "externalTool" });
  for (const forbidden of ["score", "correct", "result", "provider", "token", "launchUrl"]) {
    attempt.response = { kind: "externalTool", [forbidden]: true };
    assert.throws(
      () => decodeQuestionAttempt(attempt),
      DecodeError,
      `external submission marker must reject ${forbidden}`,
    );
  }

  attempt.response = { kind: "numeric", value: 1, score: 1 };
  assert.throws(
    () => decodeQuestionAttempt(attempt),
    DecodeError,
    "all response variants are exact",
  );
});

test("new flat-family wire shapes are exact, answer-free, and coordinate-bounded", () => {
  const block = (markdown) => ({ kind: "text", markdown });
  const option = (id, markdown) => ({ id, body: [block(markdown)] });
  const base = {
    version: "0198e000-0000-7000-8000-000000000044",
    seed: 4,
    title: "Flat family contract",
    prompt: [block("Answer the question.")],
  };
  const definitions = [
    {
      kind: "multiBlank",
      blanks: [
        { id: "first", label: [block("First blank")], matchMode: "normalized", maxLength: 40 },
        { id: "second", label: [block("Second blank")], matchMode: "exact", maxLength: 20 },
      ],
    },
    {
      kind: "matching",
      prompts: [option("dna", "DNA"), option("rna", "RNA")],
      choices: [option("deoxy", "Deoxyribose"), option("ribose", "Ribose")],
    },
    {
      kind: "hotspot",
      surface: {
        asset: "0198e000-0000-7000-8000-000000000045",
        checksum: "a".repeat(64),
      },
      description: "A labeled cell diagram",
      regions: [
        { id: "nucleus", label: [block("Nucleus")], x: 1000, y: 1000, width: 2000, height: 2000 },
        { id: "cytosol", label: [block("Cytosol")], x: 6000, y: 6000, width: 2000, height: 2000 },
      ],
      selection: { kind: "exactlyOne" },
    },
  ];

  for (const response of definitions) {
    assert.deepEqual(decodeQuestionEnvelope({ ...base, response }).response, response);
    assert.throws(
      () => decodeQuestionEnvelope({ ...base, response: { ...response, answer: "server-only" } }),
      DecodeError,
    );
  }

  const responses = [
    { kind: "multiBlank", answers: [{ slot: "first", text: "adenine" }] },
    { kind: "matching", matches: [{ prompt: "dna", choice: "deoxy" }] },
    { kind: "hotspot", points: [{ x: 2000, y: 2000 }] },
  ];
  for (const response of responses) {
    assert.deepEqual(decodeStudentResponse(response), response);
    assert.throws(() => decodeStudentResponse({ ...response, correct: true }), DecodeError);
  }

  assert.throws(
    () => decodeStudentResponse({ kind: "hotspot", points: [{ x: 10_001, y: 0 }] }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionEnvelope({
        ...base,
        response: {
          ...definitions[2],
          regions: [{ id: "bad", label: [block("Bad")], x: 9000, y: 0, width: 2000, height: 1 }],
        },
      }),
    DecodeError,
  );
});

test("issued presentation envelopes strictly project all learner families without private fields", () => {
  const block = (markdown) => ({ kind: "text", markdown });
  const choice = (id, label) => ({ id, body: [block(label)] });
  const base = {
    version: "0198e000-0000-7000-8000-000000000044",
    seed: 4,
    presentationNonce: "a".repeat(32),
    title: "Issued learner presentation",
    prompt: [block("Answer the question.")],
  };
  const cases = [
    {
      response: { kind: "singleChoice", choices: [choice("0001", "A"), choice("0002", "B")] },
      expectedKind: "multipleChoice",
    },
    {
      response: {
        kind: "multipleAnswer",
        choices: [choice("0003", "A"), choice("0004", "B")],
        minimum: 1,
        maximum: 2,
      },
      expectedKind: "multipleChoice",
    },
    { response: { kind: "fillIn", maxCharacters: 40 }, expectedKind: "shortText" },
    {
      response: {
        kind: "multiFillIn",
        blanks: [{ id: "0005", label: [block("Gene")], maxCharacters: 40 }],
      },
      expectedKind: "multiBlank",
    },
    {
      response: { kind: "numerical", maxCharacters: 128, displayedUnit: "mol" },
      expectedKind: "numeric",
    },
    {
      response: {
        kind: "matching",
        prompts: [choice("0006", "DNA"), choice("0007", "RNA")],
        choices: [choice("0008", "Deoxyribose"), choice("0009", "Ribose")],
        reuseChoices: false,
      },
      expectedKind: "matching",
    },
    {
      response: { kind: "ordering", items: [choice("000a", "First"), choice("000b", "Second")] },
      expectedKind: "ordering",
    },
    {
      response: {
        kind: "hotspot",
        surface: {
          id: "000c",
          asset: {
            asset: "0198e000-0000-7000-8000-000000000045",
            checksum: "b".repeat(64),
          },
          description: "A cell diagram",
          regions: [
            { label: [block("Nucleus")], x: 1000, y: 1000, width: 2000, height: 2000 },
            { label: [block("Cytosol")], x: 6000, y: 6000, width: 2000, height: 2000 },
          ],
        },
        minimum: 1,
        maximum: 2,
      },
      expectedKind: "hotspot",
    },
  ];

  for (const { response, expectedKind } of cases) {
    assert.equal(
      decodeIssuedPresentationEnvelope({ ...base, response }).response.kind,
      expectedKind,
    );
    assert.throws(
      () =>
        decodeIssuedPresentationEnvelope({ ...base, response: { ...response, answer: "private" } }),
      DecodeError,
    );
  }
  assert.throws(
    () =>
      decodeIssuedPresentationEnvelope({
        ...base,
        presentationNonce: "A".repeat(32),
        response: cases[0].response,
      }),
    DecodeError,
  );
});

test("external-tool launch projection is exact and cannot redirect outside the origin", () => {
  const expected = {
    launchUrl: "/api/attempts/0198e000-0000-7000-8000-000000000030/external-tool/launch",
  };
  assert.deepEqual(decodeExternalToolLaunch(expected), expected);

  for (const invalid of [
    { launchUrl: "https://provider.example/launch" },
    { launchUrl: "//provider.example/launch" },
    { launchUrl: "/\\provider.example/launch" },
    { launchUrl: "/api/launch?token=secret" },
    { launchUrl: "/api/launch#fragment" },
    { launchUrl: "/api/launch", token: "secret" },
  ]) {
    assert.throws(() => decodeExternalToolLaunch(invalid), DecodeError);
  }
});

test("external-tool submissions use the protected child route and validate outbound markers", async () => {
  const requests = [];
  const receipt = {
    accepted: true,
    attempt: {
      ...publishedProblemFixture.attempts[3],
      id: "0198e000-0000-7000-8000-000000000034",
      response: { kind: "externalTool" },
      result: null,
    },
    feedback: null,
    nextIssued: null,
    nextPending: false,
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
      requests.push(request);
      const requestPath = new URL(request.url).pathname;
      const requestAttemptId = requestPath.split("/").at(-1);
      return jsonResponse({
        ...receipt,
        attempt: requestPath.includes("/external-tool/")
          ? receipt.attempt
          : { ...receipt.attempt, id: requestAttemptId },
      });
    },
  });
  const externalAttemptId = "0198e000-0000-7000-8000-000000000034";
  await client.submitResponse(externalAttemptId, { kind: "externalTool" }, "external-key");
  await client.submitResponse(
    publishedProblemFixture.attempts[0].id,
    { kind: "multipleChoice", selected: ["carbonyl"] },
    "ordinary-key",
  );

  assert.equal(requests.length, 2);
  const external = requests[0];
  const ordinary = requests[1];
  assert.notEqual(external, undefined);
  assert.notEqual(ordinary, undefined);
  assert.equal(
    new URL(external.url).pathname,
    `/api/attempts/${externalAttemptId}/external-tool/launch/submission`,
  );
  assert.equal(
    new URL(ordinary.url).pathname,
    `/api/submissions/${publishedProblemFixture.attempts[0].id}`,
  );
  assert.equal(external.headers.get("idempotency-key"), "external-key");
  assert.equal(external.headers.get("cookie"), null);
  assert.equal(external.credentials, "same-origin");
  assert.equal(await external.text(), JSON.stringify({ response: { kind: "externalTool" } }));
  assert.equal(external.url.includes("token"), false);
  assert.equal(external.url.includes("provider"), false);

  for (const field of ["score", "provider", "token"]) {
    await assert.rejects(
      () =>
        client.submitResponse(
          externalAttemptId,
          { kind: "externalTool", [field]: "forged" },
          "bad",
        ),
      DecodeError,
    );
  }
  assert.equal(requests.length, 2, "invalid external payloads must not reach fetch");
});

test("prefetch transport is a body-free same-origin POST and treats no successor as a cache miss", async () => {
  const attempt = publishedProblemFixture.attempts[0];
  assert.notEqual(attempt, undefined, "fixture must include an attempt");
  const requests = [];
  const controller = new AbortController();
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
      requests.push(request);
      return new Response(null, { status: 204 });
    },
  });

  assert.equal(await client.prefetchNextQuestion(attempt.id, controller.signal), null);
  assert.equal(requests.length, 1);
  const request = requests[0];
  assert.notEqual(request, undefined);
  assert.equal(new URL(request.url).pathname, `/api/attempts/${attempt.id}/prefetch-next`);
  assert.equal(request.method, "POST");
  assert.equal(request.credentials, "same-origin");
  assert.equal(request.cache, "no-store");
  assert.equal(request.headers.get("accept"), "application/json");
  assert.equal(request.headers.get("content-type"), null);
  assert.equal(await request.text(), "");
  assert.equal(request.signal.aborted, false);
  controller.abort();
  assert.equal(
    request.signal.aborted,
    true,
    "the request must remain abortable through its caller signal",
  );
});

test("prefetch transport rejects hostile descriptors before a page may cache them", async () => {
  const predecessor = publishedProblemFixture.attempts[0];
  assert.notEqual(predecessor, undefined, "fixture must include an attempt");
  const envelope = issuedQuestionWireForAttempt(predecessor);
  const valid = {
    predecessor: predecessor.id,
    run: predecessor.run,
    assignmentPosition: predecessor.assignmentPosition + 1,
    questionVersion: envelope.version,
    seed: envelope.seed,
    renderedQuestionSha256: "a".repeat(64),
    envelope,
  };
  const hostilePayloads = [
    {
      name: "an envelope from another version",
      value: { ...valid, questionVersion: "0198e000-0000-7000-8000-000000000099" },
    },
    { name: "an envelope with a forged seed", value: { ...valid, seed: envelope.seed + 1 } },
    { name: "private provenance", value: { ...valid, provenance: { source: "secret" } } },
  ];

  for (const hostile of hostilePayloads) {
    const client = createHttpApiClient({
      fetch: () => Promise.resolve(jsonResponse(hostile.value)),
    });
    await assert.rejects(client.prefetchNextQuestion(predecessor.id), DecodeError, hostile.name);
  }

  const anotherAttempt = publishedProblemFixture.attempts[1];
  assert.notEqual(anotherAttempt, undefined, "fixture must include an unrelated predecessor");
  const client = createHttpApiClient({
    fetch: () => Promise.resolve(jsonResponse({ ...valid, predecessor: anotherAttempt.id })),
  });
  await assert.rejects(
    client.prefetchNextQuestion(predecessor.id),
    ApiProtocolError,
    "a well-formed descriptor for another predecessor must not escape the transport",
  );
});

test("gradebook decoder accepts only compact, internally consistent summary rows", () => {
  const fixtureRow = publishedProblemFixture.gradebook[0];
  assert.notEqual(fixtureRow, undefined, "fixture must include a gradebook summary row");
  const page = { items: [fixtureRow], nextCursor: null };
  assert.deepEqual(decodeGradebookPage(page), page);

  const inconsistentTenant = structuredClone(page);
  inconsistentTenant.items[0].summary.tenant = "0198e000-0000-7000-8000-000000000099";
  assert.throws(() => decodeGradebookPage(inconsistentTenant), DecodeError);

  const inconsistentEnrollment = structuredClone(page);
  inconsistentEnrollment.items[0].summary.enrollment = "0198e000-0000-7000-8000-000000000099";
  assert.throws(() => decodeGradebookPage(inconsistentEnrollment), DecodeError);

  const nonFiniteScore = structuredClone(page);
  nonFiniteScore.items[0].summary.bestScore = Infinity;
  assert.throws(() => decodeGradebookPage(nonFiniteScore), DecodeError);

  const extraHistory = structuredClone(page);
  extraHistory.items[0].runs = [];
  assert.throws(() => decodeGradebookPage(extraHistory), DecodeError);

  assert.throws(() => decodeGradebookPage({ ...page, offset: 0 }), DecodeError);
});

test("ordinary cursor pages enforce the shared exact, bounded browser contract", async () => {
  const response = await createMockFetch()("/api/problems");
  assert.equal(response.status, 200);
  const page = await response.json();
  assert.deepEqual(decodeCatalogPage(page), page);

  assert.throws(() => decodeCatalogPage({ ...page, answerKey: "must-not-reach-UI" }), DecodeError);
  assert.throws(
    () => decodeCatalogPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
  assert.throws(() => decodeCatalogPage({ ...page, nextCursor: "x".repeat(513) }), DecodeError);
  assert.throws(
    () =>
      decodeCatalogPage({
        ...page,
        items: [{ ...page.items[0], answerKey: "must-not-reach-UI" }],
      }),
    DecodeError,
  );
});

test("mock format validation accepts only the external-tool marker pair", async () => {
  assert.deepEqual(
    await validateResponseFormatInMock({ kind: "externalTool" }, { kind: "externalTool" }),
    { violations: [] },
  );
  assert.deepEqual(
    await validateResponseFormatInMock({ kind: "externalTool" }, { kind: "numeric", value: 1 }),
    { violations: [{ kind: "responseKindMismatch" }] },
  );
});

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
    (
      await createHttpLocalCredentialLogin({ fetch: fixtureFetch, basePath: "/ple/" })(
        "local-only-token",
      )
    ).authenticated,
    true,
  );
  await client.logout();
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
  const createdCourse = await client.createCourse({ title: "BIOC 301: Biochemistry" });
  assert.equal(createdCourse.id, fixture.course.id);
  assert.equal((await client.getCourse(fixture.course.id)).role, "student");
  assert.equal((await client.getCourseAppearance(fixture.course.id)).theme, "grass");
  const gradebook = await client.listGradebook(fixture.course.id, "next page", 25);
  assert.deepEqual(gradebook.items, fixture.gradebook);
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
  const externalToolAttemptId = "0198e000-0000-7000-8000-000000000034";
  assert.deepEqual(await client.beginExternalToolLaunch(externalToolAttemptId), {
    launchUrl: `/api/attempts/${externalToolAttemptId}/external-tool/launch`,
  });
  await assert.rejects(
    client.beginExternalToolLaunch("0198e000-0000-7000-8000-000000000030"),
    (error) => error instanceof ApiRequestError && error.status === 404,
    "HTTP client must not receive launch material for a non-external fixture attempt",
  );
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
  assert.equal(screen.course.summary.id, fixture.course.id);
  assert.equal(screen.course.appearance.theme, "grass");
  assert.equal(screen.assignment.id, fixture.assignment.id);
  assert.equal(screen.attempt.run, activeRun.id);
  assert.equal(screen.issuedQuestion.version, screen.attempt.questionVersion);
  assert.equal(screen.issuedQuestion.seed, screen.attempt.seed);
  assert.equal(screen.issuedQuestion.title, fixture.publishedProblem.metadata.title);
  assert.ok(!JSON.stringify(screen.issuedQuestion).includes('"grading"'));

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
  assert.equal(
    await client.issueProtectedAssetDelivery(fixture.assets[0].id),
    "https://objects.example.test/signed/asset?expires=12345",
  );
  assert.equal(client.assetUrl(fixture.assets[0].id), `/ple/api/assets/${fixture.assets[0].id}`);

  assert.ok(requests.every((request) => request.credentials === "same-origin"));
  assert.ok(requests.every((request) => request.cache === "no-store"));
  assert.ok(requests.some((request) => request.url.endsWith("?cursor=next+page")));
  assert.ok(
    requests.some(
      (request) =>
        request.method === "POST" &&
        request.url.endsWith(`/api/assets/${fixture.assets[0].id}/delivery`),
    ),
    "protected delivery must be an explicit POST",
  );
  assert.ok(
    requests.some(
      (request) =>
        request.url.endsWith(
          `/api/courses/${fixture.course.id}/gradebook?cursor=next+page&pageSize=25`,
        ) && !request.url.includes("offset="),
    ),
    "gradebook must use the cursor-only route without an offset",
  );
  const submission = requests.find((request) => request.url.includes("/api/submissions/"));
  assert.notEqual(submission, undefined);
  assert.equal(submission.headers.get("idempotency-key"), "stable-retry-key");
  assert.equal(submission.headers.get("content-type"), "application/json");
});

test("course creation sends only a strict public title and rejects malformed input or output", async () => {
  const requests = [];
  const fixture = publishedProblemFixture.course;
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
      requests.push(request);
      return jsonResponse({ ...fixture, role: "instructor" }, 201);
    },
  });

  const created = await client.createCourse({ title: "BIOC 301: Biochemistry" });
  assert.equal(created.role, "instructor");
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "POST");
  assert.equal(new URL(requests[0].url).pathname, "/api/courses");
  assert.deepEqual(JSON.parse(await requests[0].text()), { title: "BIOC 301: Biochemistry" });
  assert.throws(
    () => client.createCourse({ title: "   " }),
    DecodeError,
    "course creation must reject an all-whitespace title before transport",
  );
  assert.throws(
    () => client.createCourse({ title: "BIOC 301", role: "sysadmin" }),
    DecodeError,
    "course creation must reject fields outside its public request contract",
  );
});

test("gradebook pageSize rejects fractional, zero, and negative client input", () => {
  const client = createHttpApiClient({ fetch: createFixtureFetch().fixtureFetch });
  const courseId = publishedProblemFixture.course.id;
  for (const pageSize of [0, -1, 1.5]) {
    assert.throws(
      () => client.listGradebook(courseId, undefined, pageSize),
      /positive safe integer/,
    );
  }
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

test("a refresh heals one pending successor on its exact incomplete run without regrading", async () => {
  const { fixtureFetch } = createFixtureFetch();
  const run = publishedProblemFixture.runs.find((candidate) => candidate.completedAt === null);
  assert.notEqual(run, undefined, "fixture must include one incomplete run");
  const attempts = publishedProblemFixture.attempts.filter((candidate) => candidate.run === run.id);
  assert.ok(attempts.length > 0, "incomplete run must have a fixture attempt after healing");
  let healed = false;
  let resumeCalls = 0;
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const url = new URL(input.toString(), "https://client.example.test");
      if (url.pathname === `/api/runs/${run.id}/attempts`) {
        return jsonResponse({ items: healed ? attempts : [], nextCursor: null });
      }
      if (url.pathname === "/api/runs" && (init?.method ?? "GET") === "POST") {
        resumeCalls += 1;
        healed = true;
        return jsonResponse(run);
      }
      if (url.pathname.includes("/api/submissions/")) {
        throw new Error("refresh recovery must never submit or regrade");
      }
      return fixtureFetch(input, init);
    },
  });

  const screen = await client.getRunScreen(run.id);
  assert.equal(screen.run.id, run.id);
  assert.equal(screen.attempt.run, run.id);
  assert.equal(resumeCalls, 1, "one bounded resume call heals the pending successor");
});

test("hostile run enrollment data is rejected before pending-successor recovery mutates anything", async () => {
  const { fixtureFetch } = createFixtureFetch();
  const run = publishedProblemFixture.runs.find((candidate) => candidate.completedAt === null);
  assert.notEqual(run, undefined, "fixture must include one incomplete run");
  let resumeCalls = 0;
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const url = new URL(input.toString(), "https://client.example.test");
      if (url.pathname === `/api/runs/${run.id}/attempts`)
        return jsonResponse({ items: [], nextCursor: null });
      if (url.pathname === `/api/enrollments/${run.enrollment}`) {
        const response = await fixtureFetch(input, init);
        const value = await response.json();
        value.summary.tenant = "0198e000-0000-7000-8000-000000000099";
        return jsonResponse(value);
      }
      if (url.pathname === "/api/runs" && (init?.method ?? "GET") === "POST") resumeCalls += 1;
      return fixtureFetch(input, init);
    },
  });
  await assert.rejects(client.getRunScreen(run.id), ApiProtocolError);
  assert.equal(resumeCalls, 0, "untrusted enrollment data must not reach start/resume");
});

test("run-screen composition rejects an issued variant for another version or seed", async () => {
  const { fixtureFetch } = createFixtureFetch();
  for (const change of ["version", "seed"]) {
    const client = createHttpApiClient({
      fetch: async (input, init) => {
        const response = await fixtureFetch(input, init);
        if (input.toString().includes("/question")) {
          const value = await response.json();
          value[change] = change === "seed" ? 99_999 : "0198e000-0000-7000-8000-000000000099";
          return jsonResponse(value);
        }
        return response;
      },
    });
    await assert.rejects(
      client.getRunScreen(publishedProblemFixture.runs.at(-1).id),
      (error) =>
        error instanceof ApiProtocolError &&
        error.message === "Run screen issued question does not match its attempt",
    );
  }
});
