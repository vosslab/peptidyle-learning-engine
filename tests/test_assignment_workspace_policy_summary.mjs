import assert from "node:assert/strict";
import test from "node:test";

import { assignmentPolicyDraftSummary } from "../src/pages/assignment_workspace/assignment_workspace_presentation_model.ts";

const baseInput = {
  savedLifecycle: "published",
  savedCurrentState: { state: "open" },
  policies: {
    completion: { kind: "scoreAtLeast", fraction: 0.8 },
    grade: "instructorSelected",
    continuedPractice: { kind: "capped", maxAdditionalRuns: 3 },
    variation: "selectedProblemVariants",
  },
  runPolicyDraft: { completionFraction: "0.75", additionalRuns: "2" },
  disclosurePolicy: {
    score: "after_submit",
    per_item_correctness: "after_submit",
    feedback_text: "after_due",
    solution: "after_close",
    class_statistics: "never",
  },
  teachingSettings: {
    timeZone: "America/Chicago",
    lifecycle: "published",
    instructions: "Use a clear structural drawing.",
    availableAt: "2026-09-01T09:00:00.000",
    dueAt: "2026-09-08T17:00:00.000",
    closesAt: null,
    timeLimitSeconds: 900,
    attemptLimit: 2,
    lateSubmission: "markLate",
    deadlineBehavior: "autoSubmit",
  },
  timeLimitSecondsDraft: "900",
  attemptLimitDraft: "2",
};

test("current-draft summary covers every Policies-owned decision in readable copy", () => {
  const summary = assignmentPolicyDraftSummary(baseInput);
  const valueFor = (key) => summary.find((item) => item.key === key)?.value ?? "";

  assert.match(valueFor("completion"), /75%/);
  assert.match(valueFor("grade"), /Instructor-selected/);
  assert.match(valueFor("continuedPractice"), /2 additional runs/);
  assert.match(valueFor("variation"), /selected problem variants/);
  assert.match(valueFor("savedDelivery"), /open now/);
  assert.match(valueFor("lifecycle"), /Published/);
  assert.match(valueFor("lifecycle"), /Student instructions included/);
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
    runPolicyDraft: { completionFraction: "1.2", additionalRuns: "-1" },
    timeLimitSecondsDraft: "0",
    attemptLimitDraft: "many",
  });

  const completion = summary.find((item) => item.key === "completion")?.value ?? "";
  const practice = summary.find((item) => item.key === "continuedPractice")?.value ?? "";
  const schedule = summary.find((item) => item.key === "scheduleLimits")?.value ?? "";
  assert.match(completion, /needs correction/);
  assert.doesNotMatch(completion, /75%/);
  assert.match(practice, /needs correction/);
  assert.doesNotMatch(practice, /2 additional runs/);
  assert.match(schedule, /time limit needs correction/);
  assert.match(schedule, /attempt limit needs correction/);
  assert.doesNotMatch(schedule, /900s time limit|2 attempts/);
});

test("summary keeps saved effective state distinct from unsaved lifecycle decisions", () => {
  const summary = assignmentPolicyDraftSummary({
    ...baseInput,
    savedCurrentState: {
      state: "scheduled",
      availableAt: "2026-09-01T09:00:00.000",
    },
    teachingSettings: {
      ...baseInput.teachingSettings,
      lifecycle: "archived",
    },
  });
  const valueFor = (key) => summary.find((item) => item.key === key)?.value ?? "";

  assert.match(valueFor("savedDelivery"), /scheduled to open/);
  assert.match(valueFor("savedDelivery"), /America\/Chicago/);
  assert.match(valueFor("lifecycle"), /Archived/);
  assert.doesNotMatch(valueFor("savedDelivery"), /Archived/);
});
