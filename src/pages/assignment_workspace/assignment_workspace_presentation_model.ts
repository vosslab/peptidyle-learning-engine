// assignment_workspace_presentation_model.ts - Student-accessible state language for policies.

import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
import type { InstructorAssignmentRevisionDefinitionLocal } from "../../../generated/api/InstructorAssignmentRevisionDefinitionLocal";
import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import {
  nonnegativeIntegerDraft,
  optionalPositiveIntegerDraft,
  scoreFractionDraft,
  type AssignmentActivityRuleDraft,
} from "./assignment_workspace_policy_model";

export type AssignmentPolicySummaryKey =
  | "savedDelivery"
  | "assignmentCompletionRule"
  | "assignmentAttemptGradeRule"
  | "assignmentAttemptContinuationRule"
  | "questionVariationRule"
  | "disclosure"
  | "lifecycle"
  | "scheduleLimits";

export interface AssignmentPolicySummaryItem {
  readonly key: AssignmentPolicySummaryKey;
  readonly label: string;
  readonly value: string;
}

export interface AssignmentPolicyDraftSummaryInput {
  readonly savedLifecycle: InstructorAssignmentRevisionDefinitionLocal["lifecycle"];
  readonly savedCurrentState: InstructorAssignmentCurrentState;
  readonly policies: AssignmentActivityRules;
  readonly activityRuleDraft: AssignmentActivityRuleDraft;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  readonly assignmentRevisionDefinition: InstructorAssignmentRevisionDefinitionLocal;
  readonly assignmentAttemptTimeLimitSecondsDraft: string;
  readonly attemptLimitDraft: string;
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current student-access state without exposing an implementation detail. */
export function assignmentCurrentStateCopy(
  lifecycle: InstructorAssignmentRevisionDefinitionLocal["lifecycle"],
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
  if (input.policies.assignmentCompletionRule.kind === "allCorrect") {
    return "All questions correct";
  }
  if (input.policies.assignmentCompletionRule.kind === "answerAll") {
    return "Answer every question";
  }
  const threshold = scoreFractionDraft(input.activityRuleDraft.completionFraction);
  if (!threshold.valid || threshold.value === null) return "Score threshold needs correction";
  return `Score at least ${threshold.value * 100}%`;
}

function assignmentAttemptContinuationRuleSummary(
  input: AssignmentPolicyDraftSummaryInput,
): string {
  if (input.policies.assignmentAttemptContinuationRule.kind === "unlimited") {
    return "Unlimited after completion";
  }
  if (input.policies.assignmentAttemptContinuationRule.kind === "closed") {
    return "Closed after completion";
  }
  const additionalRuns = nonnegativeIntegerDraft(input.activityRuleDraft.additionalRuns);
  if (!additionalRuns.valid || additionalRuns.value === null) {
    return "Additional Assignment Attempt limit needs correction";
  }
  return `${additionalRuns.value} additional Assignment Attempt${additionalRuns.value === 1 ? "" : "s"}`;
}

function disclosureSummary(rule: StudentFeedbackReleaseRule): string {
  const timing = {
    during_attempt: "while working",
    after_submit: "after submit",
    after_due: "after due",
    after_close: "after close",
    never: "never",
  } as const;
  return [
    `Score ${timing[rule.score]}`,
    `correctness ${timing[rule.per_item_correctness]}`,
    `feedback ${timing[rule.feedback_text]}`,
    `solutions ${timing[rule.solution]}`,
    `statistics ${timing[rule.class_statistics]}`,
  ].join("; ");
}

function scheduleLimitsSummary(input: AssignmentPolicyDraftSummaryInput): string {
  const assignmentRevisionDefinition = input.assignmentRevisionDefinition;
  const timeLimit = optionalPositiveIntegerDraft(input.assignmentAttemptTimeLimitSecondsDraft);
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
    assignmentRevisionDefinition.lateWorkRule === "accept"
      ? "late work accepted"
      : assignmentRevisionDefinition.lateWorkRule === "markLate"
        ? "late work accepted and marked"
        : "late work rejected";
  return [
    `Course time zone ${assignmentRevisionDefinition.timeZone}`,
    `Available ${assignmentRevisionDefinition.availableAt === null ? "now" : displayCourseLocalTime(assignmentRevisionDefinition.availableAt)}`,
    `due ${assignmentRevisionDefinition.dueAt === null ? "not set" : displayCourseLocalTime(assignmentRevisionDefinition.dueAt)}`,
    `closes ${assignmentRevisionDefinition.closesAt === null ? "not set" : displayCourseLocalTime(assignmentRevisionDefinition.closesAt)}`,
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
  const questionVariationRule = {
    reuseQuestionsWithNewSeeds: "Keep Questions and use fresh Question Seeds",
    selectedQuestionVariants: "Use selected Question Variants",
    redrawQuestionPools: "Redraw Question Pools",
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
        input.assignmentRevisionDefinition.timeZone,
      ),
    },
    {
      key: "assignmentCompletionRule",
      label: "Assignment completion rule",
      value: completionSummary(input),
    },
    {
      key: "assignmentAttemptGradeRule",
      label: "Assignment Attempt grade rule",
      value: grade[input.policies.assignmentAttemptGradeRule],
    },
    {
      key: "assignmentAttemptContinuationRule",
      label: "Assignment Attempt continuation rule",
      value: assignmentAttemptContinuationRuleSummary(input),
    },
    {
      key: "questionVariationRule",
      label: "Question variation",
      value: questionVariationRule[input.policies.questionVariationRule],
    },
    {
      key: "disclosure",
      label: "Student disclosure",
      value: disclosureSummary(input.studentFeedbackReleaseRule),
    },
    {
      key: "lifecycle",
      label: "Draft lifecycle",
      value: `${lifecycle[input.assignmentRevisionDefinition.lifecycle]}; ${input.assignmentRevisionDefinition.instructions.trim() === "" ? "no Student instructions" : "Student instructions included"}`,
    },
    {
      key: "scheduleLimits",
      label: "Draft schedule and limits",
      value: scheduleLimitsSummary(input),
    },
  ];
}
