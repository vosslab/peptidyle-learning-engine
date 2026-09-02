// test_student_feedback_panel.mjs - permanent server-projection behavior checks for Student Feedback UI.

import assert from "node:assert/strict";
import test from "node:test";

import { studentFeedbackAnnouncement } from "../src/components/student_feedback_panel.tsx";

test("withheld and released Student Feedback announce distinct, policy-neutral states", () => {
  assert.equal(
    studentFeedbackAnnouncement({
      kind: "awaiting",
      feedback: null,
      assignmentScoringState: "current",
    }),
    "Your response was recorded. Student Feedback is not available for this response.",
  );
  assert.equal(
    studentFeedbackAnnouncement({
      kind: "released",
      feedback: { correctness: true },
      assignmentScoringState: "current",
    }),
    "Student Feedback released. Correct.",
  );
});

test("non-current scores announce recoverable Student Feedback states", () => {
  assert.equal(
    studentFeedbackAnnouncement({
      kind: "released",
      feedback: { incorrectFeedback: [{ kind: "text", markdown: "Review the peptide bond." }] },
      assignmentScoringState: "recalculating",
    }),
    "Your response was recorded. Your score is being updated.",
  );
  assert.equal(
    studentFeedbackAnnouncement({
      kind: "released",
      feedback: { incorrectFeedback: [{ kind: "text", markdown: "Review the peptide bond." }] },
      assignmentScoringState: "failed",
    }),
    "Your response was recorded. Your score is waiting for instructor review.",
  );
});

test("released Student Feedback with no disclosed fields remains neutral", () => {
  assert.equal(
    studentFeedbackAnnouncement({
      kind: "released",
      feedback: {},
      assignmentScoringState: "current",
    }),
    "Student Feedback released. Your response was recorded.",
  );
});
