import assert from "node:assert/strict";
import test from "node:test";

import { assessmentPolicyDraftSummary } from "../src/pages/assessment_workspace/assessment_workspace_presentation_model.ts";

const baseInput = {
  assessmentStatus: "released",
  savedAssessmentAvailability: { state: "available" },
  policies: {
    assessmentCompletionRule: { kind: "scoreAtLeast", fraction: 0.8 },
    assessmentAttemptGradeRule: "instructorSelected",
    assessmentAttemptContinuationRule: { kind: "capped", maxAdditionalAssessmentAttempts: 3 },
    questionPoolReuseRule: "selectAgain",
    questionVariationRule: "reuseVariation",
    assessmentAttemptResumeRule: "resumable",
    assessmentQuestionDisplayRule: "allQuestions",
    assessmentNavigationRule: "freeNavigation",
    assessmentQuestionOrderRule: "authoredOrder",
  },
  activityRuleDraft: { completionFraction: "0.75", additionalAssessmentAttempts: "2" },
  studentFeedbackReleaseRule: {
    score: "after_submit",
    per_item_correctness: "after_submit",
    submitted_response: "after_due",
    question_feedback: "after_due",
    question_answer: "after_close",
    question_answer_explanation: "after_close",
    class_statistics: "never",
  },
  assessmentAuthoredContent: {
    instructions: "Use a clear structural drawing.",
    available_at: "2026-09-01T09:00:00.000",
    due_at: "2026-09-08T17:00:00.000",
    closes_at: null,
    assessment_attempt_time_limit_seconds: 900,
    attempt_limit: 2,
    late_work_rule: "mark_late",
  },
  assessmentAttemptTimeLimitSecondsDraft: "900",
  attemptLimitDraft: "2",
};

test("Assessment policy summary covers every Properties-owned decision in readable copy", () => {
  const summary = assessmentPolicyDraftSummary(baseInput);
  const valueFor = (key) => summary.find((item) => item.key === key)?.value ?? "";

  assert.match(valueFor("assessmentCompletionRule"), /75%/);
  assert.match(valueFor("assessmentAttemptGradeRule"), /Instructor-selected/);
  assert.match(valueFor("assessmentAttemptContinuationRule"), /2 additional Assessment Attempts/);
  assert.match(valueFor("questionPoolReuseRule"), /Select Questions again/);
  assert.match(valueFor("questionVariationRule"), /previous Question Variations/);
  assert.match(valueFor("savedDelivery"), /available now/);
  assert.match(valueFor("assessmentStatus"), /Released/);
  assert.match(valueFor("assessmentStatus"), /Student instructions included/);
  const schedule = valueFor("scheduleLimits");
  assert.match(schedule, /2026-09-01 09:00/);
  assert.match(schedule, /900s time limit/);
  assert.match(schedule, /2 attempts/);
  assert.match(schedule, /Instructor time zone/);
  const disclosure = valueFor("disclosure");
  for (const category of [
    "Score",
    "correctness",
    "Question Feedback",
    "Question Answer",
    "Explanation",
    "statistics",
  ]) {
    assert.match(disclosure, new RegExp(category));
  }
});

test("Assessment policy summary surfaces invalid unsaved limits without stale values", () => {
  const summary = assessmentPolicyDraftSummary({
    ...baseInput,
    activityRuleDraft: { completionFraction: "1.2", additionalAssessmentAttempts: "-1" },
    assessmentAttemptTimeLimitSecondsDraft: "0",
    attemptLimitDraft: "many",
  });

  const completion = summary.find((item) => item.key === "assessmentCompletionRule")?.value ?? "";
  const practice =
    summary.find((item) => item.key === "assessmentAttemptContinuationRule")?.value ?? "";
  const schedule = summary.find((item) => item.key === "scheduleLimits")?.value ?? "";
  assert.match(completion, /needs correction/);
  assert.doesNotMatch(completion, /75%/);
  assert.match(practice, /needs correction/);
  assert.doesNotMatch(practice, /2 additional Assessment Attempts/);
  assert.match(schedule, /time limit needs correction/);
  assert.match(schedule, /attempt limit needs correction/);
  assert.doesNotMatch(schedule, /900s time limit|2 attempts/);
});
