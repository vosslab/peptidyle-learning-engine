import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalLocalDateAndTime,
  mergeSavedActivityRuleDraft,
  nonnegativeIntegerDraft,
  numberDraft,
  optionalPositiveIntegerDraft,
  scoreFractionDraft,
} from "../src/pages/assessment_workspace/assessment_workspace_policy_model.ts";

const policies = {
  assessmentCompletionRule: { kind: "allCorrect" },
  assessmentAttemptGradeRule: "highest",
  assessmentAttemptContinuationRule: { kind: "unlimited" },
  questionPoolReuseRule: "reuseSelection",
  questionVariationRule: "newVariation",
  assessmentAttemptResumeRule: "resumable",
  assessmentQuestionDisplayRule: "allQuestions",
  assessmentNavigationRule: "freeNavigation",
  assessmentQuestionOrderRule: "authoredOrder",
};

test("policy local-time normalization preserves valid native time precision", () => {
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00"), "2026-09-01T17:00:00.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15"), "2026-09-01T17:00:15.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.1"), "2026-09-01T17:00:15.100");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.12"), "2026-09-01T17:00:15.120");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.123"), "2026-09-01T17:00:15.123");
  assert.equal(canonicalLocalDateAndTime("2026/09/01 17:00"), null);
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

test("Assessment activity-rule number drafts accept bounded values and retain invalid raw text", () => {
  assert.deepEqual(scoreFractionDraft("0.75"), { raw: "0.75", value: 0.75, valid: true });
  assert.deepEqual(scoreFractionDraft("1.1"), { raw: "1.1", value: null, valid: false });
  assert.deepEqual(scoreFractionDraft(""), { raw: "", value: null, valid: false });
  assert.deepEqual(nonnegativeIntegerDraft("0"), { raw: "0", value: 0, valid: true });
  assert.deepEqual(nonnegativeIntegerDraft("3.5"), { raw: "3.5", value: null, valid: false });
});

test("inactive conditional Assessment activity-rule drafts survive an unrelated policy save", () => {
  const original = { completionFraction: "0.65", additionalAssessmentAttempts: "7" };
  const saved = {
    ...policies,
    assessmentCompletionRule: { kind: "answerAll" },
    assessmentAttemptContinuationRule: { kind: "unlimited" },
  };

  assert.deepEqual(mergeSavedActivityRuleDraft(original, saved), original);
});
