import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeAssignmentCapabilityViolations,
  decodeAssignmentEditorDetail,
  decodeAssignmentEditorInput,
} from "../src/api/decoders.ts";
import {
  ApiRequestError,
  ApiProtocolError,
  AssignmentConflictError,
  AssignmentValidationError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";

const assignment = publishedProblemFixture.assignment;
const input = {
  title: assignment.title,
  problems: assignment.problems,
  policies: assignment.policies,
};

function jsonResponse(value, status = 200, headers = {}) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", ...headers },
  });
}

test("assignment editor decoder accepts only the stable editable projection", () => {
  const detail = decodeAssignmentEditorDetail(assignment);
  assert.equal(detail.id, assignment.id);
  assert.equal(detail.courseId, assignment.courseId);
  assert.deepEqual(detail.problems, assignment.problems);
  assert.deepEqual(decodeAssignmentEditorInput(input), input);

  for (const forbidden of [
    "workspace",
    "question",
    "source",
    "capabilities",
    "grading",
    "answerKey",
    "feedback",
  ]) {
    assert.throws(
      () => decodeAssignmentEditorDetail({ ...assignment, [forbidden]: "server-only" }),
      DecodeError,
    );
    assert.throws(
      () => decodeAssignmentEditorInput({ ...input, [forbidden]: "forged" }),
      DecodeError,
    );
  }

  assert.throws(
    () =>
      decodeAssignmentCapabilityViolations({
        error: "assignment configuration is not supported",
        violations: [
          {
            title: assignment.title,
            reference: assignment.problems[0],
            capability: "serverGrading",
            backend: "forged",
          },
        ],
      }),
    DecodeError,
  );

  const nestedUnknownInputs = [
    {
      ...input,
      problems: [{ ...assignment.problems[0], source: "forged" }],
    },
    {
      ...input,
      policies: { ...input.policies, completion: { kind: "answerAll", forged: true } },
    },
    {
      ...input,
      policies: { ...input.policies, completion: { kind: "allCorrect", forged: true } },
    },
    {
      ...input,
      policies: {
        ...input.policies,
        completion: { kind: "scoreAtLeast", fraction: 0.8, forged: true },
      },
    },
    {
      ...input,
      policies: { ...input.policies, continuedPractice: { kind: "unlimited", forged: true } },
    },
    {
      ...input,
      policies: { ...input.policies, continuedPractice: { kind: "closed", forged: true } },
    },
    {
      ...input,
      policies: {
        ...input.policies,
        continuedPractice: { kind: "capped", maxAdditionalRuns: 2, forged: true },
      },
    },
  ];
  for (const hostileInput of nestedUnknownInputs) {
    assert.throws(() => decodeAssignmentEditorInput(hostileInput), DecodeError);
  }
});

test("assignment HTTP transport preserves the exact revisioned create/read/save boundary", async () => {
  const requests = [];
  let revision = 1;
  let current = structuredClone(assignment);
  const client = createHttpApiClient({
    fetch: async (url, init) => {
      const request = new Request(new URL(String(url), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (request.method === "POST") {
        assert.equal(path, `/api/courses/${assignment.courseId}/assignments`);
        const body = JSON.parse(await request.text());
        current = { ...current, id: "0198e000-0000-7000-8000-000000000060", ...body };
        revision = 1;
        return jsonResponse(current, 201, { etag: '"1"' });
      }
      if (request.method === "PUT") {
        assert.equal(path, `/api/courses/${current.courseId}/assignments/${current.id}`);
        assert.equal(request.headers.get("if-match"), `"${revision}"`);
        const body = JSON.parse(await request.text());
        current = { ...current, ...body };
        revision += 1;
        return jsonResponse(current, 200, { etag: `"${revision}"` });
      }
      return jsonResponse(current, 200, { etag: `"${revision}"` });
    },
  });

  const read = await client.getAssignmentEditor(assignment.id);
  const created = await client.createAssignment(assignment.courseId, input);
  const saved = await client.saveAssignment(created.courseId, created.id, input, created.revision);

  assert.equal(read.revision, '"1"');
  assert.equal(created.revision, '"1"');
  assert.equal(saved.revision, '"2"');
  assert.deepEqual(JSON.parse(await requests[1].text()), input);
  assert.deepEqual(JSON.parse(await requests[2].text()), input);
  assert.equal(requests[2].headers.get("if-match"), '"1"');
  assert.equal(new URL(requests[0].url).pathname, `/api/assignments/${assignment.id}`);
  assert.equal(
    new URL(requests[1].url).pathname,
    `/api/courses/${assignment.courseId}/assignments`,
  );
  assert.ok(requests.every((request) => request.credentials === "same-origin"));
  assert.ok(requests.every((request) => request.cache === "no-store"));
});

test("revisioned assignment responses must match the requested identity before their ETag is trusted", async () => {
  const wrongAssignment = "0198e000-0000-7000-8000-000000000099";
  const wrongCourse = "0198e000-0000-7000-8000-000000000098";
  const cases = [
    {
      call: (client) => client.getAssignmentEditor(assignment.id),
      response: { ...assignment, id: wrongAssignment },
    },
    {
      call: (client) => client.createAssignment(assignment.courseId, input),
      response: { ...assignment, courseId: wrongCourse },
    },
    {
      call: (client) => client.saveAssignment(assignment.courseId, assignment.id, input, '"1"'),
      response: { ...assignment, id: wrongAssignment, courseId: wrongCourse },
    },
  ];
  for (const identityCase of cases) {
    const client = createHttpApiClient({
      fetch: async () => jsonResponse(identityCase.response, 200, { etag: '"1"' }),
    });
    await assert.rejects(identityCase.call(client), ApiProtocolError);
  }
});

test("assignment HTTP transport distinguishes validation and stale-revision failures", async () => {
  const violation = {
    title: assignment.title,
    reference: assignment.problems[0],
    capability: "serverGrading",
  };
  const validationClient = createHttpApiClient({
    fetch: async () =>
      jsonResponse(
        { error: "assignment configuration is not supported", violations: [violation] },
        422,
      ),
  });
  await assert.rejects(
    validationClient.createAssignment(assignment.courseId, input),
    (error) => error instanceof AssignmentValidationError && error.violations.length === 1,
  );

  const staleClient = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "stale" }, 409),
  });
  await assert.rejects(
    staleClient.saveAssignment(assignment.courseId, assignment.id, input, '"1"'),
    AssignmentConflictError,
  );

  const hostileEtagClient = createHttpApiClient({ fetch: async () => jsonResponse(assignment) });
  await assert.rejects(hostileEtagClient.getAssignmentEditor(assignment.id), ApiProtocolError);
});

test("assignment mock mirrors strict body, capability report, and compare-and-swap behavior", async () => {
  const client = createMockApiClient({ assignmentAuthoring: true });
  const initial = await client.getAssignmentEditor(assignment.id);
  const saved = await client.saveAssignment(
    initial.courseId,
    initial.id,
    { ...input, title: "Revised peptide practice" },
    initial.revision,
  );
  assert.equal(saved.title, "Revised peptide practice");
  assert.equal(saved.revision, '"2"');
  await assert.rejects(
    client.saveAssignment("0198e000-0000-7000-8000-000000000099", saved.id, input, saved.revision),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
  await assert.rejects(
    client.saveAssignment(saved.courseId, saved.id, input, initial.revision),
    AssignmentConflictError,
  );

  const unsupported = {
    ...input,
    problems: [
      {
        problem: assignment.problems[0].problem,
        version: "0198e000-0000-7000-8000-000000000005",
      },
    ],
  };
  await assert.rejects(
    client.createAssignment(assignment.courseId, unsupported),
    (error) =>
      error instanceof AssignmentValidationError &&
      error.violations.map((violation) => violation.capability).join(",") ===
        "serverGrading,perQuestionTiming",
  );
});

test("default mock client denies assignment mutations before issuing authoring transport", async () => {
  const client = createMockApiClient();
  await assert.rejects(client.createAssignment(assignment.courseId, input), /not authorized/);
  await assert.rejects(
    client.saveAssignment(assignment.courseId, assignment.id, input, '"1"'),
    /not authorized/,
  );
});
