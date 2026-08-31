// test_run_completion_presentation.mjs - terminal Assignment Attempt copy and action regression.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentAttemptCompletionPresentation,
  submissionAdvanceLabel,
} from "../src/pages/run_completion_presentation.ts";

function acknowledgement(assignmentAttemptCompletion) {
  return {
    accepted: true,
    attemptId: "attempt-a",
    assignmentAttemptCompletion,
    nextIssued: null,
    nextPending: false,
  };
}

test("an exhausted incomplete Assignment Attempt records the response without claiming completion", () => {
  const presentation = assignmentAttemptCompletionPresentation("inProgress", true);

  assert.equal(presentation.heading, "Completion requirement not met");
  assert.equal(
    presentation.message,
    "Your response is recorded, but this Assignment Attempt did not meet the completion requirement.",
  );
  assert.equal(
    submissionAdvanceLabel(acknowledgement("inProgress")),
    "View Assignment Attempt status",
  );
});

test("a completed Assignment Attempt retains explicit completion copy", () => {
  const presentation = assignmentAttemptCompletionPresentation("completed", false);

  assert.equal(presentation.message, "Your completed Assignment Attempt is recorded.");
  assert.equal(
    submissionAdvanceLabel(acknowledgement("completed")),
    "View completed Assignment Attempt",
  );
});
