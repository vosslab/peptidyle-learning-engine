// assignment_workspace_presentation_model.ts - Student-accessible state language for policies.

import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { StudentDisclosurePolicy } from "../../../generated/api/StudentDisclosurePolicy";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import {
  nonnegativeIntegerDraft,
  optionalPositiveIntegerDraft,
  scoreFractionDraft,
  type AssignmentActivityRuleDraft,
} from "./assignment_workspace_policy_model";

export type AssignmentPolicySummaryKey =
  | "savedDelivery"
  | "completion"
  | "grade"
  | "continuedPractice"
  | "variation"
  | "disclosure"
  | "lifecycle"
  | "scheduleLimits"

export interface AssignmentPolicySummaryItem {
  readonly key: AssignmentPolicySummaryKey;
  readonly label: string;
  readonly value: string;
}

export interface AssignmentPolicyDraftSummaryInput {
  readonly savedLifecycle: InstructorAssignmentTeachingSettingsLocal["lifecycle"];
  readonly savedCurrentState: InstructorAssignmentCurrentState;
  readonly policies: AssignmentActivityRules;
  readonly activityRuleDraft: AssignmentActivityRuleDraft;
  readonly disclosurePolicy: StudentDisclosurePolicy;
  readonly teachingSettings: InstructorAssignmentTeachingSettingsLocal;
  readonly timeLimitSecondsDraft: string;
  readonly attemptLimitDraft: string;
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current student-access state without exposing an implementation detail. */
export function assignmentCurrentStateCopy(
  lifecycle: InstructorAssignmentTeachingSettingsLocal["lifecycle"],
  current: InstructorAssignmentCurrentState,
  timeZone: string,
): string {
  if (current.state === "draft") return "Draft. Students cannot access this assignment.";
  if (current.state === "archived") return "Archived. Students cannot access this assignment.";
  if (current.state === "scheduled") {
    return `Published, scheduled to open at ${displayCourseLocalTime(current.availableAt)} ${timeZone}.`;
  }
  if (current.state === "open") return "Published, open now.";
  if (lifecycle === "published" && current.closedAt !== null) {
    return `Published, closed since ${displayCourseLocalTime(current.closedAt)} ${timeZone}.`;
  }
  return "Closed by instructor. Students cannot start new work.";
}

function completionSummary(input: AssignmentPolicyDraftSummaryInput): string {
  if (input.policies.completion.kind === "allCorrect") return "All questions correct";
  if (input.policies.completion.kind === "answerAll") return "Answer every question";
  const threshold = scoreFractionDraft(input.activityRuleDraft.completionFraction);
  if (!threshold.valid || threshold.value === null) return "Score threshold needs correction";
  return `Score at least ${threshold.value * 100}%`;
}

function continuedPracticeSummary(input: AssignmentPolicyDraftSummaryInput): string {
  if (input.policies.continuedPractice.kind === "unlimited") return "Unlimited after completion";
  if (input.policies.continuedPractice.kind === "closed") return "Closed after completion";
  const additionalRuns = nonnegativeIntegerDraft(input.activityRuleDraft.additionalRuns);
  if (!additionalRuns.valid || additionalRuns.value === null) {
    return "Additional Assignment Attempt limit needs correction";
  }
  return `${additionalRuns.value} additional Assignment Attempt${additionalRuns.value === 1 ? "" : "s"}`;
}

function disclosureSummary(policy: StudentDisclosurePolicy): string {
  const timing = {
    during_attempt: "while working",
    after_submit: "after submit",
    after_due: "after due",
    after_close: "after close",
    never: "never",
  } as const;
  return [
    `Score ${timing[policy.score]}`,
    `correctness ${timing[policy.per_item_correctness]}`,
    `feedback ${timing[policy.feedback_text]}`,
    `solutions ${timing[policy.solution]}`,
    `statistics ${timing[policy.class_statistics]}`,
  ].join("; ");
}

function scheduleLimitsSummary(input: AssignmentPolicyDraftSummaryInput): string {
  const teaching = input.teachingSettings;
  const timeLimit = optionalPositiveIntegerDraft(input.timeLimitSecondsDraft);
  const attempts = optionalPositiveIntegerDraft(input.attemptLimitDraft);
  const timeLimitCopy = !timeLimit.valid
    ? "time limit needs correction"
    : timeLimit.value === null
      ? "no time limit"
      : `${timeLimit.value}s time limit`;
  const attemptCopy = !attempts.valid
    ? "attempt limit needs correction"
    : attempts.value === null
      ? "unlimited attempts"
      : `${attempts.value} attempt${attempts.value === 1 ? "" : "s"}`;
  const lateCopy =
    teaching.lateSubmission === "accept"
      ? "late work accepted"
      : teaching.lateSubmission === "markLate"
        ? "late work accepted and marked"
        : "late work rejected";
  return [
    `Course time zone ${teaching.timeZone}`,
    `Available ${teaching.availableAt === null ? "now" : displayCourseLocalTime(teaching.availableAt)}`,
    `due ${teaching.dueAt === null ? "not set" : displayCourseLocalTime(teaching.dueAt)}`,
    `closes ${teaching.closesAt === null ? "not set" : displayCourseLocalTime(teaching.closesAt)}`,
    timeLimitCopy,
    attemptCopy,
    lateCopy,
    "active work auto-submits at the effective deadline",
  ].join("; ");
}

/** Builds concise, student-safe copy from the current unsaved Policies draft. */
export function assignmentPolicyDraftSummary(
  input: AssignmentPolicyDraftSummaryInput,
): ReadonlyArray<AssignmentPolicySummaryItem> {
  const grade = {
    highest: "Highest Assignment Attempt score",
    latest: "Latest Assignment Attempt score",
    first: "First Assignment Attempt score",
    instructorSelected: "Instructor-selected Assignment Attempt",
  } as const;
  const variation = {
    newSeeds: "Use new seeds",
    selectedProblemVariants: "Use selected problem variants",
    fullRegeneration: "Fully regenerate",
  } as const;
  const lifecycle = {
    draft: "Draft",
    published: "Published",
    closed: "Closed",
    archived: "Archived",
  } as const;
  return [
    {
      key: "savedDelivery",
      label: "Current saved delivery",
      value: assignmentCurrentStateCopy(
        input.savedLifecycle,
        input.savedCurrentState,
        input.teachingSettings.timeZone,
      ),
    },
    { key: "completion", label: "Completion", value: completionSummary(input) },
    { key: "grade", label: "Grade", value: grade[input.policies.grade] },
    {
      key: "continuedPractice",
      label: "Continued practice",
      value: continuedPracticeSummary(input),
    },
    { key: "variation", label: "Variation", value: variation[input.policies.variation] },
    {
      key: "disclosure",
      label: "Student disclosure",
      value: disclosureSummary(input.disclosurePolicy),
    },
    {
      key: "lifecycle",
      label: "Draft lifecycle",
      value: `${lifecycle[input.teachingSettings.lifecycle]}; ${input.teachingSettings.instructions.trim() === "" ? "no Student instructions" : "Student instructions included"}`,
    },
    {
      key: "scheduleLimits",
      label: "Draft schedule and limits",
      value: scheduleLimitsSummary(input),
    },
  ];
}
