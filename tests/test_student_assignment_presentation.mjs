// Shared answer-free Student landing behavior checks.

import assert from "node:assert/strict";
import test from "node:test";

import {
  formatAssignmentAttemptTimeLimit,
  formatAssignmentActivity,
  formatAssignmentDeliveryTime,
  toStudentAssignmentPresentationData,
} from "../src/components/student_assignment_presentation.tsx";

const instructorDelivery = {
  available_at: null,
  due_at: null,
  closes_at: null,
  assignment_attempt_time_limit_seconds: 900,
  attempt_limit: 2,
  late_work_rule: "accept",
  assignment_deadline_rule: "auto_submit",
};

test("Student detail adapts available entries and Question Pool selections without exposing source identities", () => {
  const presentation = toStudentAssignmentPresentationData({
    id: "assignment-1",
    reference: "AS-1",
    title: "Protein structure",
    instructions: "Use your notes.",
    display_time_zone: "America/New_York",
    delivery: {
      available_at: null,
      due_at: null,
      closes_at: null,
      assignment_attempt_time_limit_seconds: 900,
      attempt_limit: 2,
      late_work_rule: "accept",
      assignment_deadline_rule: "auto_submit",
      student_late_work_status: "on_time",
    },
    entries: [
      { kind: "fixedQuestion", availability: "available" },
      { kind: "fixedQuestion", availability: "retired" },
      { kind: "fixedQuestion", availability: "available" },
      { kind: "questionPool", availability: "available", selectionCount: 3 },
      { kind: "questionPool", availability: "retired", selectionCount: 2 },
    ],
  });

  assert.equal(presentation.questionsPerAssignmentAttempt, 5);
  assert.equal(presentation.delivery.studentLateWorkStatus, "on_time");
  assert.equal(presentation.displayTimeZone, "America/New_York");
  assert.equal("timeZone" in presentation, false);
  assert.equal("id" in presentation, false);
});

test("Instructor Student view keeps its explicit Question Variation Rule and disclosure data", () => {
  const presentation = toStudentAssignmentPresentationData({
    title: "Protein structure",
    instructions: "Use your notes.",
    displayTimeZone: "America/Los_Angeles",
    delivery: instructorDelivery,
    questionsPerAssignmentAttempt: 4,
    questionPoolReuseRule: "selectAgain",
    questionVariationRule: "newVariation",
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      question_feedback: "after_due",
      question_answer: "after_close",
      question_answer_explanation: "after_close",
      class_statistics: "never",
    },
  });

  assert.equal(presentation.questionsPerAssignmentAttempt, 4);
  assert.equal(presentation.questionPoolReuseRule, "selectAgain");
  assert.equal(presentation.questionVariationRule, "newVariation");
  assert.equal(presentation.studentFeedbackReleaseRule?.question_feedback, "after_due");
  assert.equal(presentation.displayTimeZone, "America/Los_Angeles");
  assert.equal("studentLateWorkStatus" in presentation.delivery, false);
});

test("attempt-time copy stays readable across minute, hour, and second limits", () => {
  assert.equal(formatAssignmentAttemptTimeLimit(3_600), "1 hour per attempt");
  assert.equal(formatAssignmentAttemptTimeLimit(90), "90 seconds per attempt");
});

test("assignment instants use the supplied viewer zone instead of the browser zone", () => {
  const timestamp = Date.parse("2026-01-15T18:30:00Z");
  const newYork = "America/New_York";
  const losAngeles = "America/Los_Angeles";
  const options = { dateStyle: "medium", timeStyle: "short", timeZone: newYork };
  const expected = new Intl.DateTimeFormat(undefined, options).format(new Date(timestamp));

  assert.equal(formatAssignmentDeliveryTime(timestamp, newYork), expected);
  assert.equal(formatAssignmentActivity(timestamp, newYork), expected);
  assert.notEqual(
    formatAssignmentDeliveryTime(timestamp, losAngeles),
    expected,
    "fixed instant must render in the supplied viewer zone",
  );
});
