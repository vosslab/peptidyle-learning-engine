// Focused browser-client proof for the Policies release-readiness action.

import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  decodeAssignmentUnreleaseImpact,
  decodeLiveAssignmentWorkspace,
} from "../src/api/decoders/assignment_release.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const course = "C-1";
const assignment = "A-2";

function createdWorkspace(displayTimeZone = "America/Chicago") {
  return {
    reference: assignment,
    editNumber: "1",
    status: "unreleased",
    source: {
      blueprint_revision: { reference: "BP-1", revision: "1" },
      blueprint_assignment_reference: "00000000-0000-0000-0000-000000000011",
    },
    title: "Peptide bonds",
    instructions: "",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      assignmentCompletionRule: { kind: "answerAll" },
      assignmentAttemptGradeRule: "latest",
      assignmentAttemptContinuationRule: { kind: "closed" },
      questionPoolReuseRule: "selectAgain",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "allQuestions",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "never",
      per_item_correctness: "never",
      submitted_response: "never",
      question_feedback: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
    },
    displayTimeZone,
    entries: [
      {
        kind: "fixedQuestion",
        id: "00000000-0000-0000-0000-000000000001",
        reference: { questionId: "7K3-M9QP", revisionNumber: 1 },
        pointsPossible: "1",
        availability: "available",
        scoringRule: "normal",
        questionAttemptLimit: { maxAttempts: null },
        questionAttemptTimeLimit: { kind: "unlimited" },
      },
      {
        kind: "questionPool",
        id: "00000000-0000-0000-0000-000000000002",
        availability: "available",
        scoringRule: "normal",
        selectionCount: 1,
        pointsPerItem: "2",
        selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
        questionAttemptLimit: { maxAttempts: 2 },
        questionAttemptTimeLimit: { kind: "limited", seconds: 60, graceSeconds: 5 },
        items: [
          {
            id: "00000000-0000-0000-0000-000000000003",
            reference: { questionId: "2R5-X7YA", revisionNumber: 1 },
            availability: "available",
          },
        ],
      },
    ],
    questions: [
      {
        reference: { questionId: "7K3-M9QP", revisionNumber: 1 },
        description: "A fixed question.",
      },
    ],
  };
}

test("Assignment Workspace accepts exact browser-supported zones and rejects invalid names", async () => {
  const validFetch = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace("UTC")), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  const created = await createHttpApiClient({
    fetch: validFetch.recordingFetch,
  }).createLiveAssignment(course, {
    blueprintAssignmentReference: "00000000-0000-0000-0000-000000000011",
    title: "Peptide bonds",
    instructions: "",
  });
  assert.equal(created.workspace.displayTimeZone, "UTC");

  const invalidFetch = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace("not/a-zone")), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  await assert.rejects(
    createHttpApiClient({ fetch: invalidFetch.recordingFetch }).createLiveAssignment(course, {
      blueprintAssignmentReference: "00000000-0000-0000-0000-000000000011",
      title: "Peptide bonds",
      instructions: "",
    }),
    /response\.displayTimeZone must be a browser-supported IANA time-zone name/u,
  );
});

test("Assignment creation uses the Course Instance assignment boundary", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(createdWorkspace()), {
        status: 201,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"1"',
        },
      }),
  );

  await createHttpApiClient({ fetch: recordingFetch }).createLiveAssignment(course, {
    blueprintAssignmentReference: "00000000-0000-0000-0000-000000000011",
    title: "Peptide bonds",
    instructions: "",
  });

  assert.equal(requests[0].method, "POST");
  assert.equal(new URL(requests[0].url).pathname, "/api/course-instances/C-1/assignments");
});

test("current Assignment workspace retains exact source and normalized fixed and pool pins", () => {
  const workspace = decodeLiveAssignmentWorkspace(createdWorkspace());
  assert.equal(workspace.source.blueprint_revision.reference, "BP-1");
  assert.equal(workspace.entries[0].kind, "fixedQuestion");
  assert.equal(workspace.entries[1].kind, "questionPool");
  assert.equal(workspace.entries[1].items[0].reference.questionId, "2R5-X7YA");
  assert.throws(() => decodeLiveAssignmentWorkspace({ ...createdWorkspace(), revisionNumber: 1 }));
});

test("release returns the complete current Assignment and its replacement ETag", async () => {
  const released = { ...createdWorkspace(), status: "released", editNumber: "2" };
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(released), {
        status: 200,
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
          etag: '"2"',
        },
      }),
  );
  const result = await createHttpApiClient({ fetch: recordingFetch }).releaseLiveAssignment(
    course,
    assignment,
    '"1"',
  );
  assert.equal(result.workspace.status, "released");
  assert.equal(result.workspace.entries[1].kind, "questionPool");
  assert.equal(result.etag, '"2"');
  assert.equal(requests[0].method, "POST");
  assert.equal(requests[0].headers.get("if-match"), '"1"');
});

test("release readiness uses the current direct Assignment validation boundary", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(
        JSON.stringify({
          canRelease: false,
          issues: ["noPublishedQuestions", "timeLimitRequired"],
        }),
        {
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json; charset=utf-8",
          },
        },
      ),
  );

  const validation = await createHttpApiClient({
    fetch: recordingFetch,
  }).validateLiveAssignmentRelease(course, assignment);

  assert.deepEqual(validation, {
    canRelease: false,
    issues: ["noPublishedQuestions", "timeLimitRequired"],
  });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "GET");
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/C-1/assignments/A-2/release-validation",
  );
});

test("Unrelease uses aggregate-only impact, exact title, and a replacement Assignment ETag", async () => {
  const released = { ...createdWorkspace(), status: "released", editNumber: "2" };
  const unreleased = { ...createdWorkspace(), status: "unreleased", editNumber: "3" };
  const impact = {
    confirmationTitle: "Peptide bonds",
    editNumber: "2",
    attemptCount: 4,
    submissionCount: 6,
    gradeCount: 3,
  };
  const { recordingFetch, requests } = createRecordingFetch(async (request) => {
    const pathname = new URL(request.url).pathname;
    if (pathname.endsWith("/unrelease-impact")) {
      return new Response(JSON.stringify(impact), {
        headers: { "cache-control": "no-store", "content-type": "application/json; charset=utf-8" },
      });
    }
    return new Response(JSON.stringify({ assignment: unreleased, deleted: impact }), {
      status: 200,
      headers: {
        "cache-control": "no-store",
        "content-type": "application/json; charset=utf-8",
        etag: '"3"',
      },
    });
  });
  const client = createHttpApiClient({ fetch: recordingFetch });
  const loadedImpact = await client.getLiveAssignmentUnreleaseImpact(course, assignment);
  assert.deepEqual(loadedImpact, impact);
  const result = await client.unreleaseLiveAssignment(
    course,
    assignment,
    loadedImpact.confirmationTitle,
    '"2"',
  );
  assert.equal(result.result.assignment.status, "unreleased");
  assert.equal(result.result.deleted.submissionCount, 6);
  assert.equal(result.etag, '"3"');
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[1].method, "POST");
  assert.equal(requests[1].headers.get("if-match"), '"2"');
  assert.deepEqual(JSON.parse(await requests[1].text()), {
    confirmationTitle: "Peptide bonds",
  });
  assert.equal(released.status, "released");
});

test("Unrelease impact refuses Student detail and unknown fields", () => {
  const impact = {
    confirmationTitle: "Peptide bonds",
    editNumber: "2",
    attemptCount: 4,
    submissionCount: 6,
    gradeCount: 3,
  };
  assert.equal(decodeAssignmentUnreleaseImpact(impact).attemptCount, 4);
  assert.throws(() => decodeAssignmentUnreleaseImpact({ ...impact, studentId: "student-1" }));
  assert.throws(() => decodeAssignmentUnreleaseImpact({ ...impact, response: "secret" }));
});
