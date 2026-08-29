// assignment_workspace_presentation_model.ts - learner-accessible state language for policies.

import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { StudentDisclosurePolicy } from "../../../generated/api/StudentDisclosurePolicy";
import type { RunPolicies } from "../../../generated/api/RunPolicies";
import type { AssignmentPoliciesInput } from "../../api/contracts";
import {
  nonnegativeIntegerDraft,
  optionalPositiveIntegerDraft,
  scoreFractionDraft,
  type RunPolicyDraft,
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
  | "audience";

export interface AssignmentPolicySummaryItem {
  readonly key: AssignmentPolicySummaryKey;
  readonly label: string;
  readonly value: string;
}

export interface AssignmentPolicyDraftSummaryInput {
  readonly savedLifecycle: InstructorAssignmentTeachingSettingsLocal["lifecycle"];
  readonly savedCurrentState: InstructorAssignmentCurrentState;
  readonly policies: RunPolicies;
  readonly runPolicyDraft: RunPolicyDraft;
  readonly disclosurePolicy: StudentDisclosurePolicy;
  readonly teachingSettings: InstructorAssignmentTeachingSettingsLocal;
  readonly timeLimitSecondsDraft: string;
  readonly attemptLimitDraft: string;
  readonly audience: AssignmentPoliciesInput["audience"];
  readonly courseGroups: ReadonlyArray<{ readonly reference: string; readonly title: string }>;
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current learner-access state without exposing an implementation detail. */
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
  const threshold = scoreFractionDraft(input.runPolicyDraft.completionFraction);
  if (!threshold.valid || threshold.value === null) return "Score threshold needs correction";
  return `Score at least ${threshold.value * 100}%`;
}

function continuedPracticeSummary(input: AssignmentPolicyDraftSummaryInput): string {
  if (input.policies.continuedPractice.kind === "unlimited") return "Unlimited after completion";
  if (input.policies.continuedPractice.kind === "closed") return "Closed after completion";
  const additionalRuns = nonnegativeIntegerDraft(input.runPolicyDraft.additionalRuns);
  if (!additionalRuns.valid || additionalRuns.value === null) {
    return "Additional-run limit needs correction";
  }
  return `${additionalRuns.value} additional run${additionalRuns.value === 1 ? "" : "s"}`;
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

function audienceSummary(input: AssignmentPolicyDraftSummaryInput): string {
  if (input.audience.kind === "courseWide") return "Every enrolled learner";
  if (input.audience.groups.length === 0) return "No course groups selected";
  const titles = new Map(input.courseGroups.map((group) => [group.reference, group.title]));
  const selectedTitles = input.audience.groups.flatMap((reference) => {
    const title = titles.get(reference);
    return title === undefined ? [] : [title];
  });
  if (selectedTitles.length !== input.audience.groups.length) {
    return `${input.audience.groups.length} selected course group${input.audience.groups.length === 1 ? "" : "s"}`;
  }
  return selectedTitles.join(", ");
}

/** Builds concise, learner-safe copy from the current unsaved Policies draft. */
export function assignmentPolicyDraftSummary(
  input: AssignmentPolicyDraftSummaryInput,
): ReadonlyArray<AssignmentPolicySummaryItem> {
  const grade = {
    highest: "Highest run score",
    latest: "Latest run score",
    first: "First run score",
    instructorSelected: "Instructor-selected run",
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
      label: "Learner disclosure",
      value: disclosureSummary(input.disclosurePolicy),
    },
    {
      key: "lifecycle",
      label: "Draft lifecycle",
      value: `${lifecycle[input.teachingSettings.lifecycle]}; ${input.teachingSettings.instructions.trim() === "" ? "no learner instructions" : "learner instructions included"}`,
    },
    {
      key: "scheduleLimits",
      label: "Draft schedule and limits",
      value: scheduleLimitsSummary(input),
    },
    { key: "audience", label: "Audience", value: audienceSummary(input) },
  ];
}
