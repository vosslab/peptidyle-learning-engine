import assert from "node:assert/strict";
import test from "node:test";

import {
  AUTOMATED_GRADING_RECOVERY_LABELS,
  answerFreeViolation,
  automatedGradingRetryName,
  isInstructorRetryPost,
  isLearnerStatusGet,
  isLearnerSubmissionPost,
} from "../tests/playwright/e2e/automated_grading_recovery_ui.ts";

test("automated-grading recovery retains its visible and answer-free browser contract", () => {
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.responseReceived, "Response received");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.checkGradingStatus, "Check grading status");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.gradingOperations, "Grading operations");
  assert.equal(AUTOMATED_GRADING_RECOVERY_LABELS.gradebook, "Gradebook");
  assert.equal(automatedGradingRetryName.test("Retry grading operation GO-17"), true);
  assert.equal(automatedGradingRetryName.test("Retry grading operation GO-0"), false);
  assert.equal(isInstructorRetryPost("POST", "/grading-operations/GO-17/retry"), true);
  assert.equal(isInstructorRetryPost("GET", "/grading-operations/GO-17/retry"), false);
  assert.equal(isLearnerSubmissionPost("POST", "/api/attempts/A/submissions"), true);
  assert.equal(isLearnerSubmissionPost("GET", "/api/attempts/A/submissions"), false);
  assert.equal(isLearnerStatusGet("GET", "/api/attempts/A/submission-status"), true);
  assert.equal(isLearnerStatusGet("POST", "/api/attempts/A/submission-status"), false);
  assert.equal(answerFreeViolation({ kind: "accepted_pending", attemptId: "A-1" }), null);
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
