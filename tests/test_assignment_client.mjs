import assert from "node:assert/strict";
import test from "node:test";

import { decodeStudentAssessmentDetail } from "../src/api/decoders/assessment_teaching_delivery.ts";
import {
  decodeCourseAssessments,
  decodeSaveLiveAssessmentInlineInput,
  decodeSaveBaseAssessmentPolicyInput,
} from "../src/api/decoders/assessment_release.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { LiveAssessmentWorkspaceConflictError } from "../src/api/http_client/assessment_release.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function savedPolicyWorkspace() {
  return {
    reference: "A8H4N6P",
    editNumber: "4",
    status: "unreleased",
    assessmentType: "regular_assignment",
    origin: {
      kind: "adopted",
      source: {
        blueprint_revision: { reference: "BP7K3M2Q", revision: "1" },
        blueprint_assessment_reference: "00000000-0000-0000-0000-000000000011",
      },
    },
    title: "Peptide bonds",
    instructions: "Read carefully.",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: 60,
    attemptLimit: null,
    activityRules: {
      assessmentAttemptGradeRule: "highest",
      questionVariationRule: "newVariation",
      assessmentAttemptResumeRule: "resumable",
      assessmentQuestionDisplayRule: "oneQuestionAtATime",
      assessmentNavigationRule: "freeNavigation",
      assessmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_answer: "after_submit",
      question_answer_explanation: "after_submit",
      class_statistics: "never",
    },
    displayTimeZone: "America/Chicago",
    entries: [],
    questions: [],
  };
}

function baseAssessmentPolicy() {
  return {
    instructions: "Read carefully.",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: 60,
    attemptLimit: null,
    activityRules: {
      assessmentAttemptGradeRule: "highest",
      questionVariationRule: "newVariation",
      assessmentAttemptResumeRule: "resumable",
      assessmentQuestionDisplayRule: "oneQuestionAtATime",
      assessmentNavigationRule: "freeNavigation",
      assessmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_answer: "after_submit",
      question_answer_explanation: "after_submit",
      class_statistics: "never",
    },
  };
}

test("Course Assessment rows require exact due and Instructor-zone display facts", () => {
  const rows = decodeCourseAssessments([
    {
      reference: "A8H4N6P",
      assessmentType: "regular_assignment",
      title: "Peptide bonds",
      dueAt: "2026-09-11T14:30:00.000",
      displayTimeZone: "America/Chicago",
      status: "released",
      editNumber: "3",
    },
  ]);
  assert.equal(rows[0].dueAt, "2026-09-11T14:30:00.000");
  assert.equal(rows[0].displayTimeZone, "America/Chicago");

  assert.throws(() =>
    decodeCourseAssessments([
      {
        reference: "A8H4N6P",
        assessmentType: "regular_assignment",
        title: "Peptide bonds",
        displayTimeZone: "America/Chicago",
        status: "released",
        editNumber: "3",
      },
    ]),
  );
  assert.throws(() =>
    decodeCourseAssessments([
      {
        reference: "A8H4N6P",
        assessmentType: "regular_assignment",
        title: "Peptide bonds",
        dueAt: "2026-09-11T14:30:00.000",
        displayTimeZone: "America/Chicago",
        status: "released",
        editNumber: "3",
        extra: true,
      },
    ]),
  );
});

test("inline Assessment row saves accept only title and a required nullable local due value", () => {
  assert.deepEqual(decodeSaveLiveAssessmentInlineInput({ title: "Peptide bonds", dueAt: null }), {
    title: "Peptide bonds",
    dueAt: null,
  });
  assert.throws(() => decodeSaveLiveAssessmentInlineInput({ title: "Peptide bonds" }));
  assert.throws(() =>
    decodeSaveLiveAssessmentInlineInput({
      title: "Peptide bonds",
      dueAt: "2026-09-11T14:30",
    }),
  );
});

test("Base Assessment Policy saves are closed and cannot carry title or Entries", () => {
  const policy = baseAssessmentPolicy();
  assert.deepEqual(decodeSaveBaseAssessmentPolicyInput(policy), policy);
  assert.throws(() =>
    decodeSaveBaseAssessmentPolicyInput({ ...policy, title: "must not cross boundary" }),
  );
});

test("Base Assessment Policy save uses the current workspace boundary and exact replacement ETag", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(savedPolicyWorkspace()), {
        headers: { "cache-control": "no-store", "content-type": "application/json", etag: '"4"' },
      }),
  );

  const saved = await createHttpApiClient({ fetch: recordingFetch }).saveBaseAssessmentPolicy(
    "CI7K3M2Q",
    "A8H4N6P",
    baseAssessmentPolicy(),
    '"3"',
  );

  assert.equal(saved.workspace.editNumber, "4");
  assert.equal(saved.etag, '"4"');
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/CI7K3M2Q/assessments/A8H4N6P/policies",
  );
  assert.equal(requests[0].method, "PUT");
  assert.equal(requests[0].headers.get("if-match"), '"3"');
  assert.deepEqual(JSON.parse(await requests[0].text()), baseAssessmentPolicy());
});

test("Base Assessment Policy save requires a matching response ETag and maps an Edit Number conflict", async () => {
  const missingEtag = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(savedPolicyWorkspace()), {
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: missingEtag.recordingFetch }).saveBaseAssessmentPolicy(
      "CI7K3M2Q",
      "A8H4N6P",
      baseAssessmentPolicy(),
      '"3"',
    ),
    /ETag must match/u,
  );

  const conflict = createRecordingFetch(
    async () =>
      new Response("Assessment Workspace changed", {
        status: 412,
        headers: { "cache-control": "no-store" },
      }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: conflict.recordingFetch }).saveBaseAssessmentPolicy(
      "CI7K3M2Q",
      "A8H4N6P",
      baseAssessmentPolicy(),
      '"3"',
    ),
    LiveAssessmentWorkspaceConflictError,
  );
});

test("Student Assessment detail accepts only its viewer-owned display zone", () => {
  const detail = decodeStudentAssessmentDetail({
    id: "00000000-0000-0000-0000-000000000001",
    reference: "A9D2RX5",
    title: "Peptide bonds",
    instructions: "Use your notes.",
    display_time_zone: "America/New_York",
    delivery: {
      available_at: 1_768_502_800_000,
      due_at: 1_768_506_400_000,
      closes_at: null,
      assessment_attempt_time_limit_seconds: null,
      attempt_limit: null,
      late_work_rule: "accept",
      student_late_work_status: "on_time",
    },
    entries: [],
  });
  assert.equal(detail.display_time_zone, "America/New_York");
  assert.equal("time_zone" in detail, false);
  assert.throws(() => decodeStudentAssessmentDetail({ ...detail, time_zone: "America/Chicago" }));
  assert.throws(() => decodeStudentAssessmentDetail({ ...detail, accountId: "account-1" }));
});
