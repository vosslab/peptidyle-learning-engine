// assessment_workspace_presentation_model.ts - Student-accessible state language for policies.

import type { InstructorAssessmentAvailabilityView } from "../../../generated/api/InstructorAssessmentAvailabilityView";
import type { AssessmentStatus } from "../../../generated/api/AssessmentStatus";
import type { InstructorAssessmentAuthoredContentLocal } from "../../../generated/api/InstructorAssessmentAuthoredContentLocal";
import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type { AssessmentActivityRules } from "../../../generated/api/AssessmentActivityRules";
import { optionalPositiveIntegerDraft } from "./assessment_workspace_policy_model";

export type AssessmentPolicySummaryKey =
  | "savedDelivery"
  | "assessmentAttemptGradeRule"
  | "questionPoolReuseRule"
  | "questionVariationRule"
  | "disclosure"
  | "assessmentStatus"
  | "scheduleLimits";

export interface AssessmentPolicySummaryItem {
  readonly key: AssessmentPolicySummaryKey;
  readonly label: string;
  readonly value: string;
}

export interface AssessmentPolicyDraftSummaryInput {
  readonly assessmentStatus: AssessmentStatus;
  readonly savedAssessmentAvailability: InstructorAssessmentAvailabilityView;
  readonly policies: AssessmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  readonly assessmentAuthoredContent: InstructorAssessmentAuthoredContentLocal;
  readonly assessmentAttemptTimeLimitSecondsDraft: string;
  readonly attemptLimitDraft: string;
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current student-access state without exposing an implementation detail. */
export function assessmentAvailabilityCopy(
  status: AssessmentStatus,
  current: InstructorAssessmentAvailabilityView,
  timeZone: string,
): string {
  if (current.state === "unreleased") return "Unreleased. Students cannot access this assessment.";
  if (current.state === "archived") return "Archived. Students cannot access this assessment.";
  if (current.state === "scheduled") {
    return `Released, scheduled to open at ${displayCourseLocalTime(current.available_at)} ${timeZone}.`;
  }
  if (current.state === "available") return "Released, available now.";
  if (status === "released" && current.closed_at !== null) {
    return `Released, closed since ${displayCourseLocalTime(current.closed_at)} ${timeZone}.`;
  }
  return "Closed by instructor. Students cannot start new work.";
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
    `Question Feedback ${timing[rule.question_feedback]}`,
    `Question Answer ${timing[rule.question_answer]}`,
    `Answer Explanation ${timing[rule.question_answer_explanation]}`,
    `statistics ${timing[rule.class_statistics]}`,
  ].join("; ");
}

function scheduleLimitsSummary(input: AssessmentPolicyDraftSummaryInput): string {
  const assessmentAuthoredContent = input.assessmentAuthoredContent;
  const timeLimit = optionalPositiveIntegerDraft(input.assessmentAttemptTimeLimitSecondsDraft);
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
    assessmentAuthoredContent.late_work_rule === "accept"
      ? "late work accepted"
      : assessmentAuthoredContent.late_work_rule === "mark_late"
        ? "late work accepted and marked"
        : "late work rejected";
  return [
    "Instructor time zone",
    `Available ${assessmentAuthoredContent.available_at === null ? "now" : displayCourseLocalTime(assessmentAuthoredContent.available_at)}`,
    `due ${assessmentAuthoredContent.due_at === null ? "not set" : displayCourseLocalTime(assessmentAuthoredContent.due_at)}`,
    `closes ${assessmentAuthoredContent.closes_at === null ? "not set" : displayCourseLocalTime(assessmentAuthoredContent.closes_at)}`,
    timeLimitCopy,
    attemptCopy,
    lateCopy,
    "active work auto-submits at the effective deadline",
  ].join("; ");
}

/** Builds concise, student-safe copy from the current unsaved Policies draft. */
export function assessmentPolicyDraftSummary(
  input: AssessmentPolicyDraftSummaryInput,
): ReadonlyArray<AssessmentPolicySummaryItem> {
  const grade = {
    highest: "Highest Assessment Attempt score",
    latest: "Latest Assessment Attempt score",
    first: "First Assessment Attempt score",
    instructorSelected: "Instructor-selected Assessment Attempt",
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
      value: assessmentAvailabilityCopy(
        input.assessmentStatus,
        input.savedAssessmentAvailability,
        "your Instructor time zone",
      ),
    },
    {
      key: "assessmentAttemptGradeRule",
      label: "Assessment Attempt grade rule",
      value: grade[input.policies.assessmentAttemptGradeRule],
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
      key: "assessmentStatus",
      label: "Assessment status",
      value: `${status[input.assessmentStatus]}; ${input.assessmentAuthoredContent.instructions.trim() === "" ? "no Student instructions" : "Student instructions included"}`,
    },
    {
      key: "scheduleLimits",
      label: "Assessment schedule and limits",
      value: scheduleLimitsSummary(input),
    },
  ];
}
