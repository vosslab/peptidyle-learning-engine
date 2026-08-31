// Shared answer-free Student landing behavior checks.

import assert from "node:assert/strict";
import test from "node:test";

import {
  formatAssignmentAttemptTimeLimit,
  toStudentAssignmentPresentationData,
} from "../src/components/student_assignment_presentation.tsx";

const instructorDelivery = {
  availableAt: null,
  dueAt: null,
  closesAt: null,
  assignmentAttemptTimeLimitSeconds: 900,
  attemptLimit: 2,
  lateWorkRule: "accept",
  assignmentDeadlineRule: "autoSubmit",
};

test("Student detail adapts available entries and pool draws without exposing source identities", () => {
  const presentation = toStudentAssignmentPresentationData({
    id: "assignment-1",
    reference: "AS-1",
    title: "Protein structure",
    instructions: "Use your notes.",
    time_zone: "America/Chicago",
    delivery: {
      available_at: null,
      due_at: null,
      closes_at: null,
      assignment_attempt_time_limit_seconds: 900,
      attempt_limit: 2,
      late_work_rule: "accept",
      assignment_deadline_rule: "autoSubmit",
      late_status: "on_time",
    },
    entries: [
      { kind: "fixedQuestion", availability: "available" },
      { kind: "fixedQuestion", availability: "retired" },
      { kind: "fixedQuestion", availability: "available" },
      { kind: "questionPool", availability: "available", drawCount: 3 },
      { kind: "questionPool", availability: "retired", drawCount: 2 },
    ],
  });

  assert.equal(presentation.questionsPerRun, 5);
  assert.equal(presentation.delivery.lateStatus, "on_time");
  assert.equal("id" in presentation, false);
});

test("Instructor Student view keeps its explicit Question Variation Rule and disclosure data", () => {
  const presentation = toStudentAssignmentPresentationData({
    title: "Protein structure",
    instructions: "Use your notes.",
    timeZone: "America/Chicago",
    delivery: instructorDelivery,
    questionsPerRun: 4,
    questionVariationRule: "redrawQuestionPools",
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_due",
      solution: "after_close",
      class_statistics: "never",
    },
  });

  assert.equal(presentation.questionsPerRun, 4);
  assert.equal(presentation.questionVariationRule, "redrawQuestionPools");
  assert.equal(presentation.studentFeedbackReleaseRule?.feedback_text, "after_due");
  assert.equal("lateStatus" in presentation.delivery, false);
});

test("attempt-time copy stays readable across minute, hour, and second limits", () => {
  assert.equal(formatAssignmentAttemptTimeLimit(3_600), "1 hour per attempt");
  assert.equal(formatAssignmentAttemptTimeLimit(90), "90 seconds per attempt");
});
