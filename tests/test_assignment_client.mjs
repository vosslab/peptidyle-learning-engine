import assert from "node:assert/strict";
import test from "node:test";

import { decodeInstructorStudentView } from "../src/api/decoders/assignment_teaching_delivery.ts";

test("Instructor Student view accepts an empty draft and Question Pool redraw without identities", () => {
  const view = decodeInstructorStudentView({
    title: "Peptide bonds",
    instructions: "Add questions before publishing.",
    timeZone: "America/Chicago",
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
  assert.equal("studentLateWorkStatus" in view.delivery, false);
  assert.throws(() =>
    decodeInstructorStudentView({ ...view, assignmentId: "00000000-0000-0000-0000-000000000001" }),
  );
});
