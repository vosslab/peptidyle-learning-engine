import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentPoliciesInput,
  assignmentPolicyCanReload,
  assignmentPoliciesValidationFeedback,
  assignmentPolicyFeedbackRole,
  canonicalCourseLocalTime,
  mergeSavedActivityRuleDraft,
  nonnegativeIntegerDraft,
  numberDraft,
  optionalPositiveIntegerDraft,
  activityRuleDraftFromRules,
  scoreFractionDraft,
} from "../src/pages/assignment_workspace/assignment_workspace_policy_model.ts";

const studentFeedbackReleaseRule = {
  score: "after_submit",
  per_item_correctness: "after_submit",
  feedback_text: "after_due",
  solution: "after_close",
  class_statistics: "never",
};

const policies = {
  assignmentCompletionRule: { kind: "allCorrect" },
  assignmentAttemptGradeRule: "highest",
  assignmentAttemptContinuationRule: { kind: "unlimited" },
  questionVariationRule: "reuseQuestionsWithNewSeeds",
  assignmentAttemptResumeRule: "resumable",
  assignmentQuestionDisplayRule: "allQuestions",
  assignmentNavigationRule: "freeNavigation",
  assignmentQuestionOrderRule: "authoredOrder",
};

const assignmentRevisionDefinition = {
  timeZone: "America/Chicago",
  lifecycle: "draft",
  instructions: "Use a clear structural drawing.",
  availableAt: null,
  dueAt: "2026-09-01T17:00:00.000",
  closesAt: null,
  assignmentAttemptTimeLimitSeconds: null,
  attemptLimit: null,
  lateWorkRule: "markLate",
  assignmentDeadlineRule: "autoSubmit",
};

test("focused policy input preserves direct delivery settings", () => {
  const input = assignmentPoliciesInput(studentFeedbackReleaseRule, policies, assignmentRevisionDefinition);

  assert.equal(input.assignmentRevisionDefinition.instructions, "Use a clear structural drawing.");
});

test("policy local-time normalization accepts only explicit course wall-clock values", () => {
  assert.equal(canonicalCourseLocalTime("2026-09-01T17:00"), "2026-09-01T17:00:00.000");
  assert.equal(canonicalCourseLocalTime("2026/09/01 17:00"), null);
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
  assert.deepEqual(optionalPositiveIntegerDraft(""), { raw: "", value: null, valid: true });
  assert.deepEqual(optionalPositiveIntegerDraft("12"), { raw: "12", value: 12, valid: true });
  assert.deepEqual(optionalPositiveIntegerDraft("0"), { raw: "0", value: null, valid: false });
  assert.deepEqual(optionalPositiveIntegerDraft("2147483648"), {
    raw: "2147483648",
    value: null,
    valid: false,
  });
  assert.equal(numberDraft(90), "90");
  assert.equal(numberDraft(null), "");
});

test("Assignment activity-rule number drafts accept bounded values and retain invalid raw text", () => {
  assert.deepEqual(scoreFractionDraft("0.75"), { raw: "0.75", value: 0.75, valid: true });
  assert.deepEqual(scoreFractionDraft("1.1"), { raw: "1.1", value: null, valid: false });
  assert.deepEqual(scoreFractionDraft(""), { raw: "", value: null, valid: false });
  assert.deepEqual(nonnegativeIntegerDraft("0"), { raw: "0", value: 0, valid: true });
  assert.deepEqual(nonnegativeIntegerDraft("3.5"), { raw: "3.5", value: null, valid: false });
});

test("inactive conditional Assignment activity-rule drafts survive a successful unrelated policy save", () => {
  const original = { completionFraction: "0.65", additionalRuns: "7" };
  const saved = {
    ...policies,
    assignmentCompletionRule: { kind: "answerAll" },
    assignmentAttemptContinuationRule: { kind: "unlimited" },
  };

  assert.deepEqual(activityRuleDraftFromRules(saved), {
    completionFraction: "0.8",
    additionalRuns: "3",
  });
  assert.deepEqual(mergeSavedActivityRuleDraft(original, saved), original);
});

test("server policy issues select the first repair while keeping concise safe details", () => {
  const feedback = assignmentPoliciesValidationFeedback([
    {
      kind: "capability",
      title: "Peptide geometry",
      questionId: "7K3-M9QP",
      capability: "serverGrading",
    },
  ]);

  assert.equal(feedback.target, "questions");
  assert.equal(feedback.questionRepairRequired, true);
  assert.deepEqual(feedback.details, ["Peptide geometry needs server grading."]);
  assert.equal(JSON.stringify(feedback).includes("7K3-M9QP"), false);
});

test("draft revision publication readiness gives lifecycle focus and a Questions repair route", () => {
  const feedback = assignmentPoliciesValidationFeedback([
    { kind: "draftRevisionPublicationReadiness", blockingIssues: [{ kind: "questionsRequired" }] },
  ]);

  assert.equal(feedback.target, "lifecycle");
  assert.equal(feedback.questionRepairRequired, true);
  assert.equal(feedback.message, "Add at least one question before publishing this assignment.");
});
