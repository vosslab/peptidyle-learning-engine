import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import {
  gradingOperationsActionFailure,
  gradingOperationsSubjectLabel,
  gradingOperationsPositionForFocus,
  gradingOperationsRetryLabel,
  retryGradingOperationsAction,
  retryOperationIntent,
} from "../src/pages/assignment_workspace/assignment_workspace_operations_model.ts";

test("grading-operation focus starts with no cursor", () => {
  assert.deepEqual(gradingOperationsPositionForFocus("student"), {
    focus: "student",
    cursor: undefined,
  });
});

test("grading-operation retry preserves the original Instructor Grading Operation Retry Token", () => {
  const intent = retryOperationIntent("GO-12", 7, "00000000-0000-0000-0000-000000000012");
  assert.equal(
    retryGradingOperationsAction(intent).instructorGradingOperationRetryToken,
    intent.instructorGradingOperationRetryToken,
  );
  assert.equal(retryGradingOperationsAction(intent).expectedRevision, '"7"');
});

test("grading-operation stale conflicts require an explicit reload", () => {
  assert.deepEqual(gradingOperationsActionFailure(new ApiRequestError(412, "/operations")), {
    kind: "stale",
    message:
      "This assignment changed before the grading request was accepted. Reload the latest assignment before continuing.",
  });
});

test("grading-operation retry control names its exact recovery target", () => {
  const row = {
    subject: {
      kind: "question",
      questionId: "PEP-7B4D",
      title: "Peptide bond resonance",
    },
  };

  assert.equal(gradingOperationsSubjectLabel(row), "Question: Peptide bond resonance");
  assert.equal(
    gradingOperationsRetryLabel(row),
    "Retry automated grading for Peptide bond resonance (PEP-7B4D)",
  );
});
