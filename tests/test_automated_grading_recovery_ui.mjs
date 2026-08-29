import assert from "node:assert/strict";
import test from "node:test";

import {
  AUTOMATED_GRADING_RECOVERY_LABELS,
  automatedGradingRetryName,
  isInstructorOperationsListGet,
  isInstructorRetryPost,
  isStudentStatusGet,
  isStudentSubmissionPost,
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
  assert.equal(isStudentSubmissionPost("POST", "/api/attempts/A/submissions"), true);
  assert.equal(isStudentSubmissionPost("GET", "/api/attempts/A/submissions"), false);
  assert.equal(isStudentStatusGet("GET", "/api/attempts/A/submission-status"), true);
  assert.equal(isStudentStatusGet("POST", "/api/attempts/A/submission-status"), false);
});
