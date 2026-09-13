import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeInstructorStudentView,
  decodeStudentAssignmentDetail,
} from "../src/api/decoders/assignment_teaching_delivery.ts";
import {
  decodeCourseAssignments as decodeCourseAssignmentRows,
  decodeCourseAssignmentSourceChoices,
  decodeSaveLiveAssignmentInlineInput,
  decodeSaveBaseAssignmentPolicyInput,
} from "../src/api/decoders/assignment_release.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { LiveAssignmentWorkspaceConflictError } from "../src/api/http_client/assignment_release.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function savedPolicyWorkspace() {
  return {
    reference: "A-2",
    editNumber: "4",
    status: "unreleased",
    source: {
      blueprint_revision: { reference: "BP-1", revision: "1" },
      blueprint_assignment_reference: "00000000-0000-0000-0000-000000000011",
    },
    title: "Peptide bonds",
    instructions: "Read carefully.",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: 60,
    attemptLimit: null,
    activityRules: {
      assignmentCompletionRule: { kind: "answerAll" },
      assignmentAttemptGradeRule: "highest",
      assignmentAttemptContinuationRule: { kind: "unlimited" },
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "oneQuestionAtATime",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_feedback: "after_submit",
      question_answer: "after_submit",
      question_answer_explanation: "after_submit",
      class_statistics: "never",
    },
    displayTimeZone: "America/Chicago",
    entries: [],
    questions: [],
  };
}

function baseAssignmentPolicy() {
  return {
    instructions: "Read carefully.",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assignmentAttemptTimeLimitSeconds: 60,
    attemptLimit: null,
    activityRules: {
      assignmentCompletionRule: { kind: "answerAll" },
      assignmentAttemptGradeRule: "highest",
      assignmentAttemptContinuationRule: { kind: "unlimited" },
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "oneQuestionAtATime",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_feedback: "after_submit",
      question_answer: "after_submit",
      question_answer_explanation: "after_submit",
      class_statistics: "never",
    },
  };
}

test("Course Assignment rows require exact due and Instructor-zone display facts", () => {
  const rows = decodeCourseAssignmentRows([
    {
      reference: "A-2",
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
    decodeCourseAssignmentRows([
      {
        reference: "A-2",
        title: "Peptide bonds",
        displayTimeZone: "America/Chicago",
        status: "released",
        editNumber: "3",
      },
    ]),
  );
  assert.throws(() =>
    decodeCourseAssignmentRows([
      {
        reference: "A-2",
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

test("inline Assignment row saves accept only title and a required nullable local due value", () => {
  assert.deepEqual(decodeSaveLiveAssignmentInlineInput({ title: "Peptide bonds", dueAt: null }), {
    title: "Peptide bonds",
    dueAt: null,
  });
  assert.throws(() => decodeSaveLiveAssignmentInlineInput({ title: "Peptide bonds" }));
  assert.throws(() =>
    decodeSaveLiveAssignmentInlineInput({
      title: "Peptide bonds",
      dueAt: "2026-09-11T14:30",
    }),
  );
});

test("Base Assignment Policy saves are closed and cannot carry title or Entries", () => {
  const policy = baseAssignmentPolicy();
  assert.deepEqual(decodeSaveBaseAssignmentPolicyInput(policy), policy);
  assert.throws(() =>
    decodeSaveBaseAssignmentPolicyInput({ ...policy, title: "must not cross boundary" }),
  );
});

test("Base Assignment Policy save uses the current workspace boundary and exact replacement ETag", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(savedPolicyWorkspace()), {
        headers: { "cache-control": "no-store", "content-type": "application/json", etag: '"4"' },
      }),
  );

  const saved = await createHttpApiClient({ fetch: recordingFetch }).saveBaseAssignmentPolicy(
    "C-1",
    "A-2",
    baseAssignmentPolicy(),
    '"3"',
  );

  assert.equal(saved.workspace.editNumber, "4");
  assert.equal(saved.etag, '"4"');
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/C-1/assignments/A-2/policies",
  );
  assert.equal(requests[0].method, "PUT");
  assert.equal(requests[0].headers.get("if-match"), '"3"');
  assert.deepEqual(JSON.parse(await requests[0].text()), baseAssignmentPolicy());
});

test("Base Assignment Policy save requires a matching response ETag and maps an Edit Number conflict", async () => {
  const missingEtag = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(savedPolicyWorkspace()), {
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: missingEtag.recordingFetch }).saveBaseAssignmentPolicy(
      "C-1",
      "A-2",
      baseAssignmentPolicy(),
      '"3"',
    ),
    /ETag must match/u,
  );

  const conflict = createRecordingFetch(
    async () =>
      new Response("Assignment Workspace changed", {
        status: 412,
        headers: { "cache-control": "no-store" },
      }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: conflict.recordingFetch }).saveBaseAssignmentPolicy(
      "C-1",
      "A-2",
      baseAssignmentPolicy(),
      '"3"',
    ),
    LiveAssignmentWorkspaceConflictError,
  );
});

test("Course Assignment source choices retain the Course-pinned exact Blueprint Revision", () => {
  const choices = decodeCourseAssignmentSourceChoices([
    {
      source: {
        blueprint_revision: { reference: "BP-4", revision: "2" },
        blueprint_assignment_reference: "00000000-0000-0000-0000-000000000005",
      },
      label: "Genetics - Mendelian inheritance - Punnett squares",
    },
  ]);
  assert.equal(choices[0].source.blueprint_revision.reference, "BP-4");
  assert.equal(choices[0].source.blueprint_revision.revision, "2");
  assert.equal(
    choices[0].source.blueprint_assignment_reference,
    "00000000-0000-0000-0000-000000000005",
  );
  assert.throws(() =>
    decodeCourseAssignmentSourceChoices([
      {
        ...choices[0],
        source: { ...choices[0].source, unexpected: true },
      },
    ]),
  );
  assert.throws(() =>
    decodeCourseAssignmentSourceChoices([
      {
        ...choices[0],
        label: " ",
      },
    ]),
  );
});

test("Instructor Student view accepts an empty draft and Question Pool redraw without identities", () => {
  const view = decodeInstructorStudentView({
    title: "Peptide bonds",
    instructions: "Add questions before publishing.",
    displayTimeZone: "America/Los_Angeles",
    delivery: {
      available_at: null,
      due_at: null,
      closes_at: null,
      assignment_attempt_time_limit_seconds: null,
      attempt_limit: null,
      late_work_rule: "accept",
    },
    questionsPerAssignmentAttempt: 0,
    questionPoolReuseRule: "selectAgain",
    questionVariationRule: "newVariation",
    studentFeedbackReleaseRule: {
      score: "never",
      per_item_correctness: "never",
      submitted_response: "never",
      question_feedback: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
    },
  });
  assert.equal(view.questionsPerAssignmentAttempt, 0);
  assert.equal(view.questionPoolReuseRule, "selectAgain");
  assert.equal(view.questionVariationRule, "newVariation");
  assert.equal(view.studentFeedbackReleaseRule.submitted_response, "never");
  assert.equal(view.displayTimeZone, "America/Los_Angeles");
  assert.equal("timeZone" in view, false);
  assert.equal("studentLateWorkStatus" in view.delivery, false);
  const { submitted_response: _submittedResponse, ...withoutSubmittedResponse } =
    view.studentFeedbackReleaseRule;
  assert.throws(() =>
    decodeInstructorStudentView({
      ...view,
      studentFeedbackReleaseRule: withoutSubmittedResponse,
    }),
  );
  assert.throws(() =>
    decodeInstructorStudentView({ ...view, assignmentId: "00000000-0000-0000-0000-000000000001" }),
  );
});

test("Student assignment detail accepts only its viewer-owned display zone", () => {
  const detail = decodeStudentAssignmentDetail({
    id: "00000000-0000-0000-0000-000000000001",
    reference: "A-1",
    title: "Peptide bonds",
    instructions: "Use your notes.",
    display_time_zone: "America/New_York",
    delivery: {
      available_at: 1_768_502_800_000,
      due_at: 1_768_506_400_000,
      closes_at: null,
      assignment_attempt_time_limit_seconds: null,
      attempt_limit: null,
      late_work_rule: "accept",
      student_late_work_status: "on_time",
    },
    entries: [],
  });
  assert.equal(detail.display_time_zone, "America/New_York");
  assert.equal("time_zone" in detail, false);
  assert.throws(() => decodeStudentAssignmentDetail({ ...detail, time_zone: "America/Chicago" }));
  assert.throws(() => decodeStudentAssignmentDetail({ ...detail, accountId: "account-1" }));
});
