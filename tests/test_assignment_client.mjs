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
} from "../src/api/decoders/assignment_release.ts";

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
