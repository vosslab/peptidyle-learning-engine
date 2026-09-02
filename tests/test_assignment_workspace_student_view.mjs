import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import {
  STUDENT_VIEW_CUE,
  STUDENT_VIEW_ENTRY_PATH,
  studentViewFailureState,
} from "../src/pages/assignment_workspace/assignment_workspace_student_view_model.ts";
import { toStudentAssignmentPresentationData } from "../src/components/student_assignment_presentation.tsx";

test("Student view keeps its exact mode cue and explicit live-demo entry path", () => {
  assert.equal(
    STUDENT_VIEW_CUE,
    "Student view - current live assignment. Use Student entry to submit graded work.",
  );
  assert.equal(STUDENT_VIEW_ENTRY_PATH, "/sign-in");
});

test("Student view hides authorization failures while allowing retryable errors", () => {
  assert.equal(studentViewFailureState(new ApiRequestError(404, "/student-view")), "unavailable");
  assert.equal(studentViewFailureState(new ApiRequestError(403, "/student-view")), "unavailable");
  assert.equal(studentViewFailureState(new Error("temporary transport failure")), "error");
});

test("Student view presentation stays answer-free and preserves live delivery facts", () => {
  const presentation = toStudentAssignmentPresentationData({
    title: "Protein structure",
    instructions: "Use your notes.",
    timeZone: "America/Chicago",
    delivery: {
      availableAt: null,
      dueAt: null,
      closesAt: null,
      assignmentAttemptTimeLimitSeconds: 900,
      attemptLimit: 2,
      lateWorkRule: "markLate",
      assignmentDeadlineRule: "autoSubmit",
    },
    questionsPerAssignmentAttempt: 3,
    questionPoolReuseRule: "reuseSelection",
    questionVariationRule: "newVariation",
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_due",
      question_answer: "after_close",
      question_answer_explanation: "after_close",
      class_statistics: "never",
    },
  });

  assert.equal(presentation.title, "Protein structure");
  assert.equal(presentation.questionsPerAssignmentAttempt, 3);
  assert.equal(presentation.delivery.lateWorkRule, "markLate");
  assert.equal("assignmentId" in presentation, false);
  assert.equal("run" in presentation, false);
});
