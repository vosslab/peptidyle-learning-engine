// assignment_workspace_presentation_model.ts - Student-accessible state language for policies.

import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
import type { AssignmentStatus } from "../../../generated/api/AssignmentStatus";
import type { InstructorAssignmentWorkingCopyDefinitionLocal } from "../../../generated/api/InstructorAssignmentWorkingCopyDefinitionLocal";
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
  | "questionPoolReuseRule"
  | "questionVariationRule"
  | "disclosure"
  | "assignmentStatus"
  | "scheduleLimits";

export interface AssignmentPolicySummaryItem {
  readonly key: AssignmentPolicySummaryKey;
  readonly label: string;
  readonly value: string;
}

export interface AssignmentPolicyDraftSummaryInput {
  readonly assignmentStatus: AssignmentStatus;
  readonly savedCurrentState: InstructorAssignmentCurrentState;
  readonly policies: AssignmentActivityRules;
  readonly activityRuleDraft: AssignmentActivityRuleDraft;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  readonly assignmentWorkingCopyDefinition: InstructorAssignmentWorkingCopyDefinitionLocal;
  readonly assignmentAttemptTimeLimitSecondsDraft: string;
  readonly attemptLimitDraft: string;
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current student-access state without exposing an implementation detail. */
export function assignmentCurrentStateCopy(
  status: AssignmentStatus,
  current: InstructorAssignmentCurrentState,
  timeZone: string,
): string {
  if (current.state === "draft") return "Draft. Students cannot access this assignment.";
  if (current.state === "archived") return "Archived. Students cannot access this assignment.";
  if (current.state === "scheduled") {
    return `Released, scheduled to open at ${displayCourseLocalTime(current.availableAt)} ${timeZone}.`;
  }
  if (current.state === "open") return "Released, open now.";
  if (status === "released" && current.closedAt !== null) {
    return `Released, closed since ${displayCourseLocalTime(current.closedAt)} ${timeZone}.`;
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
    `Question Answer ${timing[rule.question_answer]}`,
    `Answer Explanation ${timing[rule.question_answer_explanation]}`,
    `statistics ${timing[rule.class_statistics]}`,
  ].join("; ");
}

function scheduleLimitsSummary(input: AssignmentPolicyDraftSummaryInput): string {
  const assignmentWorkingCopyDefinition = input.assignmentWorkingCopyDefinition;
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
    assignmentWorkingCopyDefinition.lateWorkRule === "accept"
      ? "late work accepted"
      : assignmentWorkingCopyDefinition.lateWorkRule === "markLate"
        ? "late work accepted and marked"
        : "late work rejected";
  return [
    `Course time zone ${assignmentWorkingCopyDefinition.timeZone}`,
    `Available ${assignmentWorkingCopyDefinition.availableAt === null ? "now" : displayCourseLocalTime(assignmentWorkingCopyDefinition.availableAt)}`,
    `due ${assignmentWorkingCopyDefinition.dueAt === null ? "not set" : displayCourseLocalTime(assignmentWorkingCopyDefinition.dueAt)}`,
    `closes ${assignmentWorkingCopyDefinition.closesAt === null ? "not set" : displayCourseLocalTime(assignmentWorkingCopyDefinition.closesAt)}`,
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
  const questionPoolReuseRule = {
    reuseSelection: "Reuse the previous Question Pool Selection",
    selectAgain: "Select Questions again from each Question Pool",
  } as const;
  const questionVariationRule = {
    reuseVariation: "Reuse the previous Question Variations",
    newVariation: "Use new Question Variations",
  } as const;
  const status = {
    unreleased: "Unreleased",
    released: "Released",
    closed: "Closed",
    archived: "Archived",
  } as const;
  return [
    {
      key: "savedDelivery",
      label: "Current saved delivery",
      value: assignmentCurrentStateCopy(
        input.assignmentStatus,
        input.savedCurrentState,
        input.assignmentWorkingCopyDefinition.timeZone,
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
      key: "questionPoolReuseRule",
      label: "Question Pool reuse",
      value: questionPoolReuseRule[input.policies.questionPoolReuseRule],
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
      key: "assignmentStatus",
      label: "Assignment status",
      value: `${status[input.assignmentStatus]}; ${input.assignmentWorkingCopyDefinition.instructions.trim() === "" ? "no Student instructions" : "Student instructions included"}`,
    },
    {
      key: "scheduleLimits",
      label: "Working Copy schedule and limits",
      value: scheduleLimitsSummary(input),
    },
  ];
}
