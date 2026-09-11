import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeInstructorStudentView,
  decodeStudentAssignmentDetail,
} from "../src/api/decoders/assignment_teaching_delivery.ts";

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
      assignment_deadline_rule: "auto_submit",
    },
    questionsPerAssignmentAttempt: 0,
    questionPoolReuseRule: "selectAgain",
    questionVariationRule: "newVariation",
    studentFeedbackReleaseRule: {
      score: "never",
      per_item_correctness: "never",
      question_feedback: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
    },
  });
  assert.equal(view.questionsPerAssignmentAttempt, 0);
  assert.equal(view.questionPoolReuseRule, "selectAgain");
  assert.equal(view.questionVariationRule, "newVariation");
  assert.equal(view.displayTimeZone, "America/Los_Angeles");
  assert.equal("timeZone" in view, false);
  assert.equal("studentLateWorkStatus" in view.delivery, false);
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
      assignment_deadline_rule: "auto_submit",
      student_late_work_status: "on_time",
    },
    entries: [],
  });
  assert.equal(detail.display_time_zone, "America/New_York");
  assert.equal("time_zone" in detail, false);
  assert.throws(() => decodeStudentAssignmentDetail({ ...detail, time_zone: "America/Chicago" }));
  assert.throws(() => decodeStudentAssignmentDetail({ ...detail, accountId: "account-1" }));
});
