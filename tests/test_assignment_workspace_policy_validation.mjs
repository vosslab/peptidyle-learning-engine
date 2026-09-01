// Policies validation transport remains a strict, browser-safe aggregate boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeAssignmentPoliciesValidationFailure } from "../src/api/decoders.ts";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentPoliciesValidationError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { createRecordingFetch, jsonResponse } from "./http_client_test_support.mjs";

const course = "0198e000-0000-7000-8000-000000000001";
const assignment = "0198e000-0000-7000-8000-000000000002";
const input = {
  studentFeedbackReleaseRule: {
    score: "after_submit",
    per_item_correctness: "after_submit",
    feedback_text: "after_due",
    question_answer: "after_close",
    question_answer_explanation: "after_close",
    class_statistics: "never",
  },
  policies: {
    assignmentCompletionRule: { kind: "allCorrect" },
    assignmentAttemptGradeRule: "highest",
    assignmentAttemptContinuationRule: { kind: "unlimited" },
    questionPoolReuseRule: "reuseSelection",
    questionVariationRule: "newVariation",
    assignmentAttemptResumeRule: "resumable",
    assignmentQuestionDisplayRule: "allQuestions",
    assignmentNavigationRule: "freeNavigation",
    assignmentQuestionOrderRule: "authoredOrder",
  },
  assignmentAuthoredContent: {
    timeZone: "America/Chicago",
    instructions: "Use a structural drawing.",
    availableAt: null,
    dueAt: null,
    closesAt: null,
    assignmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    lateWorkRule: "markLate",
    assignmentDeadlineRule: "autoSubmit",
  },
};

const validationFailure = {
  error: "assignmentPoliciesInvalid",
  issues: [
    {
      kind: "capability",
      title: "Peptide geometry",
      questionId: "7K3-M9QP",
      capability: "serverGrading",
    },
  ],
};

function policySave(response) {
  return createHttpApiClient({ fetch: async () => response }).saveAssignmentPolicies(
    course,
    assignment,
    "A-1",
    input,
    '"1"',
  );
}

test("Policies save binds the reviewed Assignment Edit Number", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ error: "assignment changed" }, 412),
  );

  await assert.rejects(
    createHttpApiClient({ fetch: recordingFetch }).saveAssignmentPolicies(
      course,
      assignment,
      "A-1",
      input,
      '"1"',
    ),
    AssignmentConflictError,
  );

  assert.equal(requests.length, 1);
  const request = requests[0];
  assert.equal(request.headers.get("if-match"), '"1"');
  assert.deepEqual(await request.json(), {
    ...input,
    baseEditNumber: "1",
  });
});

test("Policies validation decoder accepts only the closed bounded envelope", () => {
  assert.deepEqual(decodeAssignmentPoliciesValidationFailure(validationFailure), validationFailure);
  assert.throws(
    () =>
      decodeAssignmentPoliciesValidationFailure({
        ...validationFailure,
        issues: [{ ...validationFailure.issues[0], internalId: "private" }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeAssignmentPoliciesValidationFailure({
        ...validationFailure,
        issues: [{ kind: "futurePolicyRule" }],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeAssignmentPoliciesValidationFailure({ ...validationFailure, issues: [] }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeAssignmentPoliciesValidationFailure({
        ...validationFailure,
        issues: [{ kind: "assignmentReleaseRequirements", blockingIssues: [] }],
      }),
    DecodeError,
  );
});

test("Policies save classifies only the exact 422 validation envelope", async () => {
  await assert.rejects(policySave(jsonResponse(validationFailure, 422)), (error) => {
    assert.ok(error instanceof AssignmentPoliciesValidationError);
    assert.deepEqual(error.issues, validationFailure.issues);
    return true;
  });
  await assert.rejects(policySave(jsonResponse({ error: "opaque" }, 422)), (error) => {
    assert.ok(error instanceof ApiRequestError);
    assert.ok(!(error instanceof AssignmentPoliciesValidationError));
    return true;
  });
});

test("Policies save retains the established conflict classes", async () => {
  for (const status of [409, 412, 428]) {
    await assert.rejects(policySave(jsonResponse({ error: "changed" }, status)), (error) => {
      assert.ok(error instanceof AssignmentConflictError);
      assert.equal(error.status, status);
      return true;
    });
  }
});
