import assert from "node:assert/strict";
import test from "node:test";

import { assignmentPolicyDraftSummary } from "../src/pages/assignment_workspace/assignment_workspace_presentation_model.ts";

const baseInput = {
  assignmentStatus: "released",
  savedCurrentState: { state: "open" },
  policies: {
    assignmentCompletionRule: { kind: "scoreAtLeast", fraction: 0.8 },
    assignmentAttemptGradeRule: "instructorSelected",
    assignmentAttemptContinuationRule: { kind: "capped", maxAdditionalRuns: 3 },
    questionVariationRule: "selectedQuestionVariants",
    assignmentAttemptResumeRule: "resumable",
    assignmentQuestionDisplayRule: "allQuestions",
    assignmentNavigationRule: "freeNavigation",
    assignmentQuestionOrderRule: "authoredOrder",
  },
  activityRuleDraft: { completionFraction: "0.75", additionalRuns: "2" },
  studentFeedbackReleaseRule: {
    score: "after_submit",
    per_item_correctness: "after_submit",
    feedback_text: "after_due",
    solution: "after_close",
    class_statistics: "never",
  },
  assignmentWorkingCopyDefinition: {
    timeZone: "America/Chicago",
    instructions: "Use a clear structural drawing.",
    availableAt: "2026-09-01T09:00:00.000",
    dueAt: "2026-09-08T17:00:00.000",
    closesAt: null,
    assignmentAttemptTimeLimitSeconds: 900,
    attemptLimit: 2,
    lateWorkRule: "markLate",
    assignmentDeadlineRule: "autoSubmit",
  },
  assignmentAttemptTimeLimitSecondsDraft: "900",
  attemptLimitDraft: "2",
};

test("current-draft summary covers every Policies-owned decision in readable copy", () => {
  const summary = assignmentPolicyDraftSummary(baseInput);
  const valueFor = (key) => summary.find((item) => item.key === key)?.value ?? "";

  assert.match(valueFor("assignmentCompletionRule"), /75%/);
  assert.match(valueFor("assignmentAttemptGradeRule"), /Instructor-selected/);
  assert.match(valueFor("assignmentAttemptContinuationRule"), /2 additional Assignment Attempts/);
  assert.match(valueFor("questionVariationRule"), /selected Question Variants/);
  assert.match(valueFor("savedDelivery"), /open now/);
  assert.match(valueFor("assignmentStatus"), /Released/);
  assert.match(valueFor("assignmentStatus"), /Student instructions included/);
  const schedule = valueFor("scheduleLimits");
  assert.match(schedule, /2026-09-01 09:00/);
  assert.match(schedule, /900s time limit/);
  assert.match(schedule, /2 attempts/);
  assert.match(schedule, /America\/Chicago/);
  assert.match(schedule, /auto-submits.*effective deadline/);
  const disclosure = valueFor("disclosure");
  for (const category of ["Score", "correctness", "feedback", "solutions", "statistics"]) {
    assert.match(disclosure, new RegExp(category));
  }
});

test("current-draft summary surfaces invalid unsaved limits without stale values", () => {
  const summary = assignmentPolicyDraftSummary({
    ...baseInput,
    activityRuleDraft: { completionFraction: "1.2", additionalRuns: "-1" },
    assignmentAttemptTimeLimitSecondsDraft: "0",
    attemptLimitDraft: "many",
  });

  const completion = summary.find((item) => item.key === "assignmentCompletionRule")?.value ?? "";
  const practice =
    summary.find((item) => item.key === "assignmentAttemptContinuationRule")?.value ?? "";
  const schedule = summary.find((item) => item.key === "scheduleLimits")?.value ?? "";
  assert.match(completion, /needs correction/);
  assert.doesNotMatch(completion, /75%/);
  assert.match(practice, /needs correction/);
  assert.doesNotMatch(practice, /2 additional Assignment Attempts/);
  assert.match(schedule, /time limit needs correction/);
  assert.match(schedule, /attempt limit needs correction/);
  assert.doesNotMatch(schedule, /900s time limit|2 attempts/);
});

test("summary keeps saved effective state distinct from Assignment Status", () => {
  const summary = assignmentPolicyDraftSummary({
    ...baseInput,
    savedCurrentState: {
      state: "scheduled",
      availableAt: "2026-09-01T09:00:00.000",
    },
    assignmentStatus: "archived",
  });
  const valueFor = (key) => summary.find((item) => item.key === key)?.value ?? "";

  assert.match(valueFor("savedDelivery"), /scheduled to open/);
  assert.match(valueFor("savedDelivery"), /America\/Chicago/);
  assert.match(valueFor("assignmentStatus"), /Archived/);
  assert.doesNotMatch(valueFor("savedDelivery"), /Archived/);
});
