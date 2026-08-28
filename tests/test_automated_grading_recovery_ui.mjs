import assert from "node:assert/strict";
import test from "node:test";

import {
  AUTOMATED_GRADING_RECOVERY_LABELS,
  answerFreeViolation,
  automatedGradingRetryName,
  completedLearnerReceiptViolation,
  isInstructorOperationsListGet,
  isInstructorRetryPost,
  isLearnerStatusGet,
  isLearnerSubmissionPost,
} from "../tests/playwright/e2e/automated_grading_recovery_ui.ts";

test("automated-grading recovery retains its visible and answer-free browser contract", () => {
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.responseReceived, "Response received");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.checkGradingStatus, "Check grading status");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.gradingOperations, "Grading operations");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.gradebook, "Gradebook");
  assert.equal(
    automatedGradingRetryName.test("Retry automated grading for Peptide bond resonance (PEP-7B4D)"),
    true,
  );
  assert.equal(automatedGradingRetryName.test("Retry grading operation GO-17"), false);
  assert.equal(isInstructorRetryPost("POST", "/grading-operations/GO-17/retry"), true);
  assert.equal(isInstructorRetryPost("GET", "/grading-operations/GO-17/retry"), false);
  assert.equal(
    isInstructorOperationsListGet("GET", "/api/courses/C-1/assignments/A-1/grading-operations"),
    true,
  );
  assert.equal(
    isInstructorOperationsListGet(
      "GET",
      "/api/courses/C-1/assignments/A-1/grading-operations/recalculate",
    ),
    false,
  );
  assert.equal(
    isInstructorOperationsListGet("POST", "/api/courses/C-1/assignments/A-1/grading-operations"),
    false,
  );
  assert.equal(isLearnerSubmissionPost("POST", "/api/attempts/A/submissions"), true);
  assert.equal(isLearnerSubmissionPost("GET", "/api/attempts/A/submissions"), false);
  assert.equal(isLearnerStatusGet("GET", "/api/attempts/A/submission-status"), true);
  assert.equal(isLearnerStatusGet("POST", "/api/attempts/A/submission-status"), false);
  assert.equal(answerFreeViolation({ kind: "accepted_pending", attemptId: "A-1" }), null);
  assert.equal(answerFreeViolation({ kind: "retry", resultingOperationRevision: 2 }), null);
  assert.match(
    answerFreeViolation({ kind: "accepted_pending", score: null }) ?? "",
    /private answer field/u,
  );
  assert.match(
    answerFreeViolation({ kind: "accepted_pending", note: "private answer" }, ["private answer"]) ??
      "",
    /private answer value/u,
  );
});

test("answer-free audit rejects compact, camel-case, and delimiter-separated private fields", () => {
  for (const key of ["grading", "gradingPayload", "privateGrading", "points_earned"]) {
    assert.match(answerFreeViolation({ [key]: "hidden" }) ?? "", /private answer field/u);
  }
});

test("completed learner audit allows disclosed feedback and rejects submitted material", () => {
  const completed = {
    kind: "completed",
    attempt: { response: null },
    feedback: { correctness: true, pointsEarned: 1, pointsPossible: 1 },
  };
  assert.equal(completedLearnerReceiptViolation(completed), null);
  assert.match(
    completedLearnerReceiptViolation({
      ...completed,
      attempt: { ...completed.attempt, response: { kind: "text", value: "private" } },
    }) ?? "",
    /submitted learner response/u,
  );
});
