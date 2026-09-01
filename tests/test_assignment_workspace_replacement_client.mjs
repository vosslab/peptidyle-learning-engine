// Focused browser-client boundary for an immediate fixed-slot replacement.

import assert from "node:assert/strict";
import test from "node:test";

import {
  ApiProtocolError,
  AssignmentConflictError,
  createHttpApiClient,
  resolveAssignmentFixedItemReplacementFailure,
} from "../src/api/http_client.ts";
import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const course = "0198e000-0000-7000-8000-000000000001";
const assignment = "0198e000-0000-7000-8000-000000000002";
const item = "0198e000-0000-7000-8000-000000000003";

function replacementEditorDetail() {
  const { disclosurePolicy: _retiredDisclosurePolicy, ...assignmentDetail } =
    publishedProblemFixture.assignment;
  const replacement = {
    ...assignmentDetail,
    id: assignment,
    courseId: course,
    entries: [
      {
        ...publishedProblemFixture.assignment.entries[0],
        questionId: "1A2-B3CD",
        title: "Replacement peptide bond question",
      },
    ],
    assignmentStatus: "unreleased",
    assignmentAuthoredContent: {
      timeZone: "America/Chicago",
      instructions: "Use a structural drawing.",
      availableAt: null,
      dueAt: null,
      closesAt: null,
      assignmentAttemptTimeLimitSeconds: null,
      attemptLimit: null,
      lateWorkRule: "accept",
      assignmentDeadlineRule: "autoSubmit",
    },
    assignmentAvailability: { state: "unreleased" },
    assignmentReleaseValidation: { blockingIssues: [] },
  };
  return replacement;
}

test("Create Assignment posts directly to the course Assignment collection", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(replacementEditorDetail()), {
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json",
          etag: '"5"',
        },
      }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  const created = await client.createAssignment(course, { title: "Protein folding practice" });

  assert.equal(created.id, assignment);
  assert.equal(requests.length, 1);
  const request = requests[0];
  assert.ok(request);
  assert.equal(new URL(request.url).pathname, `/api/courses/${course}/assignments`);
  assert.equal(request.method, "POST");
  assert.equal(request.credentials, "same-origin");
  assert.equal(request.cache, "no-store");
  assert.deepEqual(await request.json(), { title: "Protein folding practice" });
});

test("fixed-item replacement sends one edit-number-checked focused request", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify({ error: "assignment changed" }), {
        status: 412,
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  await assert.rejects(
    client.replaceAssignmentFixedItem(course, assignment, "A-1", item, "7K3-M9QP", '"4"'),
    AssignmentConflictError,
  );

  assert.equal(requests.length, 1);
  const request = requests[0];
  assert.equal(request.method, "PUT");
  assert.equal(
    new URL(request.url).pathname,
    `/api/courses/${course}/assignments/${assignment}/fixed-items/${item}`,
  );
  assert.equal(request.headers.get("if-match"), '"4"');
  assert.equal(request.credentials, "same-origin");
  assert.equal(request.cache, "no-store");
  assert.deepEqual(await request.json(), {
    baseEditNumber: "4",
    questionId: "7K3-M9QP",
  });

  await assert.rejects(
    client.replaceAssignmentFixedItem(course, assignment, "A-1", item, "7K3-M9QP", '"4"'),
    (error) => {
      assert.ok(error instanceof AssignmentConflictError);
      assert.equal(error.status, 412);
      assert.equal(resolveAssignmentFixedItemReplacementFailure(error).kind, "staleRevision");
      return true;
    },
  );
});

test("fixed-item replacement decodes the revised editor detail and its new ETag", async () => {
  const client = createHttpApiClient({
    fetch: async (_input, init) => {
      assert.equal(init?.credentials, "same-origin");
      assert.equal(init?.cache, "no-store");
      return new Response(JSON.stringify(replacementEditorDetail()), {
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json",
          etag: '"5"',
        },
      });
    },
  });

  const replaced = await client.replaceAssignmentFixedItem(
    course,
    assignment,
    "A-1",
    item,
    "1A2-B3CD",
    '"4"',
  );

  assert.equal(replaced.id, assignment);
  assert.equal(replaced.courseId, course);
  assert.equal(replaced.entries[0].id, publishedProblemFixture.assignment.entries[0].id);
  assert.equal(replaced.entries[0].questionId, "1A2-B3CD");
  assert.equal(replaced.revision, '"5"');
});

test("fixed-item replacement rejects a cacheable editor response", async () => {
  const client = createHttpApiClient({
    fetch: async () =>
      new Response(JSON.stringify(replacementEditorDetail()), {
        headers: { "content-type": "application/json", etag: '"5"' },
      }),
  });

  await assert.rejects(
    client.replaceAssignmentFixedItem(course, assignment, "A-1", item, "1A2-B3CD", '"4"'),
    (error) => error instanceof ApiProtocolError && error.message.includes("must be no-store"),
  );
});

test("fixed-item replacement refuses invalid browser locators before transport", async () => {
  const client = createHttpApiClient({
    fetch: async () => {
      throw new Error("invalid request reached transport");
    },
  });

  await assert.rejects(
    client.replaceAssignmentFixedItem(course, assignment, "A-1", "", "7K3-M9QP", '"4"'),
    ApiProtocolError,
  );
});
