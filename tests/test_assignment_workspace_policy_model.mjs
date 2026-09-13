import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalLocalDateAndTime,
  DEFAULT_DUE_TIME,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
  mergeSavedActivityRuleDraft,
  nonnegativeIntegerDraft,
  numberDraft,
  optionalPositiveIntegerDraft,
  activityRuleDraftFromRules,
  scoreFractionDraft,
} from "../src/pages/assignment_workspace/assignment_workspace_policy_model.ts";

const policies = {
  assignmentCompletionRule: { kind: "allCorrect" },
  assignmentAttemptGradeRule: "highest",
  assignmentAttemptContinuationRule: { kind: "unlimited" },
  questionPoolReuseRule: "reuseSelection",
  questionVariationRule: "newVariation",
  assignmentAttemptResumeRule: "resumable",
  assignmentQuestionDisplayRule: "allQuestions",
  assignmentNavigationRule: "freeNavigation",
  assignmentQuestionOrderRule: "authoredOrder",
};

test("policy local-time normalization preserves valid native time precision", () => {
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00"), "2026-09-01T17:00:00.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15"), "2026-09-01T17:00:15.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.1"), "2026-09-01T17:00:15.100");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.12"), "2026-09-01T17:00:15.120");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.123"), "2026-09-01T17:00:15.123");
  assert.equal(canonicalLocalDateAndTime("2026/09/01 17:00"), null);
});

test("a selected due date starts at the teaching default while authored times stay exact", () => {
  assert.equal(DEFAULT_DUE_TIME, "23:59:00.000");
  assert.equal(dueDateDraft(null), "");
  assert.equal(dueTimeDraft(null), DEFAULT_DUE_TIME);
  assert.equal(localDueDateAndTime("2026-12-01", dueTimeDraft(null)), "2026-12-01T23:59:00.000");
  assert.equal(dueDateDraft("2026-12-01T12:34:56.789"), "2026-12-01");
  assert.equal(dueTimeDraft("2026-12-01T12:34:56.789"), "12:34:56.789");
  assert.equal(localDueDateAndTime("", DEFAULT_DUE_TIME), "");
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
  const original = { completionFraction: "0.65", additionalAssignmentAttempts: "7" };
  const saved = {
    ...policies,
    assignmentCompletionRule: { kind: "answerAll" },
    assignmentAttemptContinuationRule: { kind: "unlimited" },
  };

  assert.deepEqual(activityRuleDraftFromRules(saved), {
    completionFraction: "0.8",
    additionalAssignmentAttempts: "3",
  });
  assert.deepEqual(mergeSavedActivityRuleDraft(original, saved), original);
});
