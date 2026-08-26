import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentPoliciesInput,
  assignmentPolicyCanReload,
  assignmentPolicyFeedbackRole,
  canonicalCourseLocalTime,
  hasEmptyGroupAudience,
  numberDraft,
  positiveIntegerDraft,
} from "../src/pages/assignment_workspace/assignment_workspace_policy_model.ts";

const disclosurePolicy = {
  score: "afterSubmit",
  perItemCorrectness: "afterSubmit",
  feedbackText: "afterDue",
  solution: "afterClose",
  classStatistics: "never",
};

const policies = {
  completion: { kind: "allCorrect" },
  grade: "highest",
  continuedPractice: { kind: "unlimited" },
  variation: "newSeeds",
};

const teachingSettings = {
  timeZone: "America/Chicago",
  lifecycle: "draft",
  instructions: "Use a clear structural drawing.",
  availableAt: null,
  dueAt: "2026-09-01T17:00:00.000",
  closesAt: null,
  timeLimitSeconds: null,
  attemptLimit: null,
  lateSubmission: "markLate",
  deadlineBehavior: "autoSubmit",
};

test("focused policy input preserves a selected group audience and delivery settings", () => {
  const input = assignmentPoliciesInput(
    { kind: "anyOfGroups", groups: ["HONORS-SECTION"] },
    disclosurePolicy,
    policies,
    teachingSettings,
  );

  assert.deepEqual(input.audience, { kind: "anyOfGroups", groups: ["HONORS-SECTION"] });
  assert.equal(input.teachingSettings.instructions, "Use a clear structural drawing.");
});

test("policy local-time normalization accepts only explicit course wall-clock values", () => {
  assert.equal(canonicalCourseLocalTime("2026-09-01T17:00"), "2026-09-01T17:00:00.000");
  assert.equal(canonicalCourseLocalTime("2026/09/01 17:00"), null);
});

test("an empty group audience is held locally before a focused policy save", () => {
  assert.equal(hasEmptyGroupAudience({ kind: "anyOfGroups", groups: [] }), true);
  assert.equal(hasEmptyGroupAudience({ kind: "courseWide" }), false);
});

test("policy feedback makes save failures and conflicts actionable while successes stay quiet", () => {
  assert.equal(
    assignmentPolicyFeedbackRole({ kind: "error", message: "Fix this field." }),
    "alert",
  );
  assert.equal(assignmentPolicyFeedbackRole({ kind: "conflict", message: "Reload." }), "alert");
  assert.equal(assignmentPolicyFeedbackRole({ kind: "success", message: "Saved." }), "status");
  assert.equal(assignmentPolicyFeedbackRole({ kind: "info", message: "Loaded." }), "status");
  assert.equal(assignmentPolicyCanReload({ kind: "conflict", message: "Reload." }), true);
  assert.equal(assignmentPolicyCanReload({ kind: "error", message: "Fix this field." }), false);
});

test("numeric policy drafts preserve invalid text and provide no stale payload value", () => {
  assert.deepEqual(positiveIntegerDraft("12"), { raw: "12", value: 12, valid: true });
  assert.deepEqual(positiveIntegerDraft(""), { raw: "", value: null, valid: true });
  assert.deepEqual(positiveIntegerDraft("0"), { raw: "0", value: null, valid: false });
  assert.deepEqual(positiveIntegerDraft("2147483648"), {
    raw: "2147483648",
    value: null,
    valid: false,
  });
  assert.equal(numberDraft(90), "90");
  assert.equal(numberDraft(null), "");
});
