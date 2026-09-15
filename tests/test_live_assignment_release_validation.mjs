// Focused browser-client proof for the Policies release-readiness action.

import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  decodeAssessmentUnreleaseImpact,
  decodeLiveAssessmentWorkspace,
} from "../src/api/decoders/assessment_release.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const course = "CI7K3M2Q";
const assessment = "A8H4N6P";

function createdWorkspace(displayTimeZone = "America/Chicago") {
  return {
    reference: assessment,
    editNumber: "1",
    status: "unreleased",
    source: {
      blueprint_revision: { reference: "BP7K3M2Q", revision: "1" },
      blueprint_assessment_reference: "00000000-0000-0000-0000-000000000011",
    },
    title: "Peptide bonds",
    instructions: "",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      assessmentCompletionRule: { kind: "answerAll" },
      assessmentAttemptGradeRule: "latest",
      assessmentAttemptContinuationRule: { kind: "closed" },
      questionPoolReuseRule: "selectAgain",
      questionVariationRule: "newVariation",
      assessmentAttemptResumeRule: "resumable",
      assessmentQuestionDisplayRule: "allQuestions",
      assessmentNavigationRule: "freeNavigation",
      assessmentQuestionOrderRule: "authoredOrder",
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
        reference: { questionId: "7K3M-X9QP", revisionNumber: 1 },
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
            reference: { questionId: "2R5X-Z7YA", revisionNumber: 1 },
            availability: "available",
          },
        ],
      },
    ],
    questions: [
      {
        reference: { questionId: "7K3M-X9QP", revisionNumber: 1 },
        description: "A fixed question.",
      },
    ],
  };
}

test("Assessment Workspace accepts exact browser-supported zones and rejects invalid names", async () => {
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
  }).createLiveAssessment(course, {
    blueprintAssessmentReference: "00000000-0000-0000-0000-000000000011",
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
    createHttpApiClient({ fetch: invalidFetch.recordingFetch }).createLiveAssessment(course, {
      blueprintAssessmentReference: "00000000-0000-0000-0000-000000000011",
      title: "Peptide bonds",
      instructions: "",
    }),
    /response\.displayTimeZone must be a browser-supported IANA time-zone name/u,
  );
});

test("Assessment creation uses the Course Instance Assessment boundary", async () => {
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

  await createHttpApiClient({ fetch: recordingFetch }).createLiveAssessment(course, {
    blueprintAssessmentReference: "00000000-0000-0000-0000-000000000011",
    title: "Peptide bonds",
    instructions: "",
  });

  assert.equal(requests[0].method, "POST");
  assert.equal(new URL(requests[0].url).pathname, "/api/course-instances/CI7K3M2Q/assessments");
});

test("current Assessment workspace retains exact source and normalized fixed and pool pins", () => {
  const workspace = decodeLiveAssessmentWorkspace(createdWorkspace());
  assert.equal(workspace.source.blueprint_revision.reference, "BP7K3M2Q");
  assert.equal(workspace.entries[0].kind, "fixedQuestion");
  assert.equal(workspace.entries[1].kind, "questionPool");
  assert.equal(workspace.entries[1].items[0].reference.questionId, "2R5X-Z7YA");
  assert.throws(() => decodeLiveAssessmentWorkspace({ ...createdWorkspace(), revisionNumber: 1 }));
});

test("release returns the complete current Assessment and its replacement ETag", async () => {
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
  const result = await createHttpApiClient({ fetch: recordingFetch }).releaseLiveAssessment(
    course,
    assessment,
    '"1"',
  );
  assert.equal(result.workspace.status, "released");
  assert.equal(result.workspace.entries[1].kind, "questionPool");
  assert.equal(result.etag, '"2"');
  assert.equal(requests[0].method, "POST");
  assert.equal(requests[0].headers.get("if-match"), '"1"');
});

test("release readiness uses the current direct Assessment validation boundary", async () => {
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
  }).validateLiveAssessmentRelease(course, assessment);

  assert.deepEqual(validation, {
    canRelease: false,
    issues: ["noPublishedQuestions", "timeLimitRequired"],
  });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "GET");
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/CI7K3M2Q/assessments/A8H4N6P/release-validation",
  );
});

test("Unrelease uses aggregate-only impact, exact title, and a replacement Assessment ETag", async () => {
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
    return new Response(JSON.stringify({ assessment: unreleased, deleted: impact }), {
      status: 200,
      headers: {
        "cache-control": "no-store",
        "content-type": "application/json; charset=utf-8",
        etag: '"3"',
      },
    });
  });
  const client = createHttpApiClient({ fetch: recordingFetch });
  const loadedImpact = await client.getLiveAssessmentUnreleaseImpact(course, assessment);
  assert.deepEqual(loadedImpact, impact);
  const result = await client.unreleaseLiveAssessment(
    course,
    assessment,
    loadedImpact.confirmationTitle,
    '"2"',
  );
  assert.equal(result.result.assessment.status, "unreleased");
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
  assert.equal(decodeAssessmentUnreleaseImpact(impact).attemptCount, 4);
  assert.throws(() => decodeAssessmentUnreleaseImpact({ ...impact, studentId: "student-1" }));
  assert.throws(() => decodeAssessmentUnreleaseImpact({ ...impact, response: "secret" }));
});
