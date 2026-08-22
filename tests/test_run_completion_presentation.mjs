// test_run_completion_presentation.mjs - terminal run copy and action regression.

import assert from "node:assert/strict";
import test from "node:test";

import {
  runCompletionPresentation,
  submissionAdvanceLabel,
} from "../src/pages/run_completion_presentation.ts";

function acknowledgement(runCompletionStatus) {
  return {
    accepted: true,
    attemptId: "attempt-a",
    runCompletionStatus,
    nextIssued: null,
    nextPending: false,
  };
}

test("an exhausted incomplete run records the response without claiming completion", () => {
  const presentation = runCompletionPresentation("inProgress", true);

  assert.equal(presentation.heading, "Completion requirement not met");
  assert.equal(
    presentation.message,
    "Your response is recorded, but this run did not meet the completion requirement.",
  );
  assert.equal(submissionAdvanceLabel(acknowledgement("inProgress")), "View run status");
});

test("a completed run retains explicit completion copy", () => {
  const presentation = runCompletionPresentation("completed", false);

  assert.equal(presentation.message, "Your completed run is recorded.");
  assert.equal(submissionAdvanceLabel(acknowledgement("completed")), "View completed run");
});
