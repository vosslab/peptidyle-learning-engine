// assignment_workspace_policy_model.ts - policy-page request construction and local control values.

import type { InstructorAssignmentAuthoredContentLocal } from "../../../generated/api/InstructorAssignmentAuthoredContentLocal";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { AssignmentPoliciesValidationIssue } from "../../../generated/api/AssignmentPoliciesValidationIssue";
import type { AssignmentPoliciesInput } from "../../api/contracts";

/** The teaching default applied when an Instructor chooses a new due date. */
export const DEFAULT_DUE_TIME = "23:59:00.000";

export type PolicyFocusTarget =
  | "instructions"
  | "availableAt"
  | "dueAt"
  | "closesAt"
  | "assignmentAttemptTimeLimitSeconds"
  | "attemptLimit"
  | "completionFraction"
  | "additionalAssignmentAttempts"
  | "questionPoolReuseRule"
  | "questionVariationRule"
  | "questions"
  | "schedule";

export type AssignmentPolicyErrorFeedback = {
  readonly kind: "error";
  readonly message: string;
  readonly target?: PolicyFocusTarget;
  readonly details?: ReadonlyArray<string>;
  readonly questionRepairRequired?: boolean;
};

export type AssignmentPolicySaveFeedback = AssignmentPolicyErrorFeedback & {
  readonly target: PolicyFocusTarget;
  readonly details: ReadonlyArray<string>;
  readonly questionRepairRequired: boolean;
};

export type AssignmentPolicyFeedback =
  | { readonly kind: "success"; readonly message: string }
  | { readonly kind: "info"; readonly message: string }
  | AssignmentPolicyErrorFeedback
  | { readonly kind: "conflict"; readonly message: string };

export function assignmentPolicyFeedbackRole(
  feedback: AssignmentPolicyFeedback | undefined,
): "alert" | "status" {
  return feedback?.kind === "error" || feedback?.kind === "conflict" ? "alert" : "status";
}

export function assignmentPolicyCanReload(feedback: AssignmentPolicyFeedback | undefined): boolean {
  return feedback?.kind === "conflict";
}

export function assignmentPolicyFeedbackDetails(
  feedback: AssignmentPolicyFeedback,
): ReadonlyArray<string> {
  return feedback.kind === "error" ? (feedback.details ?? []) : [];
}

export function assignmentPolicyFeedbackNeedsQuestionRepair(
  feedback: AssignmentPolicyFeedback | undefined,
): boolean {
  return feedback?.kind === "error" && feedback.questionRepairRequired === true;
}

/** Keeps the page's focused write closed to the policies-owned aggregate slice. */
export function assignmentPoliciesInput(
  studentFeedbackReleaseRule: AssignmentPoliciesInput["studentFeedbackReleaseRule"],
  policies: AssignmentPoliciesInput["policies"],
  assignmentAuthoredContent: InstructorAssignmentAuthoredContentLocal,
): AssignmentPoliciesInput {
  return { studentFeedbackReleaseRule, policies, assignmentAuthoredContent };
}

/** Converts a native local-date-time control value to the explicit wire form. */
export function canonicalLocalDateAndTime(value: string): string | null {
  if (value === "") return null;
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/u.exec(value);
  if (match === null) return null;
  const [, dateAndMinute, seconds = "00", fraction = ""] = match;
  return `${dateAndMinute}:${seconds}.${fraction.padEnd(3, "0")}`;
}

/** Returns the date portion of a stored local deadline for the date control. */
export function dueDateDraft(value: string | null): string {
  return value === null ? "" : value.slice(0, 10);
}

/** Returns the authored time or the teaching default for a newly selected date. */
export function dueTimeDraft(value: string | null): string {
  return value === null ? DEFAULT_DUE_TIME : value.slice(11);
}

/** Combines local date and time controls without converting them to an instant. */
export function localDueDateAndTime(date: string, time: string): string {
  return date === "" ? "" : `${date}T${time}`;
}

function assignmentAuthoredContentTarget(
  field: AssignmentPoliciesValidationIssue & { kind: "assignmentAuthoredContent" },
): PolicyFocusTarget {
  const target = field.correction.field;
  if (target === "instructions") return "instructions";
  if (target === "availableAt") return "availableAt";
  if (target === "dueAt") return "dueAt";
  if (target === "closesAt") return "closesAt";
  if (target === "assignmentAttemptTimeLimitSeconds") return "assignmentAttemptTimeLimitSeconds";
  if (target === "attemptLimit") return "attemptLimit";
  return "schedule";
}

function assignmentAuthoredContentMessage(
  reason: AssignmentPoliciesValidationIssue & { kind: "assignmentAuthoredContent" },
): string {
  switch (reason.correction.reason) {
    case "outsideCourseTerm":
      return "Choose assignment times within this course term.";
    case "nonexistentLocalTime":
    case "ambiguousLocalTime":
    case "timestampOutOfRange":
      return "Choose a valid local date and time.";
    case "scheduleOutOfOrder":
      return "Arrange the available, due, and close times in order.";
    case "assignmentAttemptTimeLimitOutOfRange":
      return "Choose a valid whole Assignment Attempt time limit.";
    case "attemptLimitOutOfRange":
      return "Choose a valid attempt limit.";
    case "invalidInstructions":
      return "Revise the Student instructions before saving.";
    case "invalidInput":
      return "Review the assignment delivery settings before saving.";
  }
}

function capabilityLabel(
  capability: AssignmentPoliciesValidationIssue & { kind: "capability" },
): string {
  switch (capability.capability) {
    case "algorithmicGeneration":
      return "algorithmic generation";
    case "clientRendering":
      return "browser rendering";
    case "serverGrading":
      return "server grading";
    case "partialCredit":
      return "partial credit";
    case "hints":
      return "hints";
    case "questionAttemptTimeLimit":
      return "per-question timing";
    case "printExport":
      return "print export";
    case "offlinePreview":
      return "offline preview";
  }
}

function issueFeedback(issue: AssignmentPoliciesValidationIssue): AssignmentPolicySaveFeedback {
  switch (issue.kind) {
    case "assignmentAuthoredContent":
      return {
        kind: "error",
        message: assignmentAuthoredContentMessage(issue),
        target: assignmentAuthoredContentTarget(issue),
        details: [],
        questionRepairRequired: false,
      };
    case "capability": {
      const variationTarget = issue.capability === "algorithmicGeneration";
      const detail = `${issue.questionTitle} needs ${capabilityLabel(issue)}.`;
      return {
        kind: "error",
        message: variationTarget
          ? "The Question Variation Rule needs a compatible Question."
          : "One or more assignment questions need attention.",
        target: variationTarget ? "questionVariationRule" : "questions",
        details: [detail],
        questionRepairRequired: !variationTarget,
      };
    }
    case "assignmentReleaseRequirements":
      return {
        kind: "error",
        message: "Add at least one question before releasing this assignment.",
        target: "questions",
        details: issue.blockingIssues.map(() => "This assignment needs at least one question."),
        questionRepairRequired: true,
      };
  }
}

/** Projects the closed server validation list into concise, actionable instructor feedback. */
export function assignmentPoliciesValidationFeedback(
  issues: ReadonlyArray<AssignmentPoliciesValidationIssue>,
): AssignmentPolicySaveFeedback {
  const first = issues[0];
  if (first === undefined) {
    return {
      kind: "error",
      message: "Review the assignment policies before saving.",
      target: "schedule",
      details: [],
      questionRepairRequired: false,
    };
  }
  const firstFeedback = issueFeedback(first);
  const details = issues.flatMap((issue, index) => {
    const feedback = issueFeedback(issue);
    if (index === 0) return feedback.details;
    return [feedback.message, ...feedback.details];
  });
  const questionRepairRequired = issues.some(
    (issue) => issueFeedback(issue).questionRepairRequired,
  );
  return { ...firstFeedback, details, questionRepairRequired };
}

export type PositiveIntegerDraft = {
  readonly raw: string;
  readonly value: number | null;
  readonly valid: boolean;
};

export type AssignmentActivityRuleDraftField =
  "completionFraction" | "additionalAssignmentAttempts";

/** Raw number controls stay local until their typed policy value is valid. */
export type AssignmentActivityRuleDraft = {
  readonly completionFraction: string;
  readonly additionalAssignmentAttempts: string;
};

export type FractionDraft = {
  readonly raw: string;
  readonly value: number | null;
  readonly valid: boolean;
};

/**
 * Parses an optional positive whole-number control.
 *
 * An empty control intentionally maps to null so the focused policy save can
 * clear an existing server-side time or attempt limit. Invalid text stays
 * local and never replaces the last valid payload value.
 */
export function optionalPositiveIntegerDraft(raw: string): PositiveIntegerDraft {
  if (raw === "") return { raw, value: null, valid: true };
  if (!/^[1-9][0-9]*$/u.test(raw)) return { raw, value: null, valid: false };
  const value = Number(raw);
  return Number.isSafeInteger(value) && value <= 2_147_483_647
    ? { raw, value, valid: true }
    : { raw, value: null, valid: false };
}

/** Accepts the decimal threshold syntax supported by the native number control. */
export function scoreFractionDraft(raw: string): FractionDraft {
  if (!/^(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)$/u.test(raw)) {
    return { raw, value: null, valid: false };
  }
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0 && value <= 1
    ? { raw, value, valid: true }
    : { raw, value: null, valid: false };
}

/** Additional practice Assignment Attempts are a bounded nonnegative whole-number setting. */
export function nonnegativeIntegerDraft(raw: string): PositiveIntegerDraft {
  if (!/^[0-9]+$/u.test(raw)) return { raw, value: null, valid: false };
  const value = Number(raw);
  return Number.isSafeInteger(value) && value <= 2_147_483_647
    ? { raw, value, valid: true }
    : { raw, value: null, valid: false };
}

export function activityRuleDraftFromRules(
  policies: AssignmentActivityRules,
): AssignmentActivityRuleDraft {
  return {
    completionFraction: numberDraft(
      policies.assignmentCompletionRule.kind === "scoreAtLeast"
        ? policies.assignmentCompletionRule.fraction
        : 0.8,
    ),
    additionalAssignmentAttempts: numberDraft(
      policies.assignmentAttemptContinuationRule.kind === "capped"
        ? policies.assignmentAttemptContinuationRule.maxAdditionalAssignmentAttempts
        : 3,
    ),
  };
}

/** Keeps inactive conditional text available for a later deliberate policy change. */
export function mergeSavedActivityRuleDraft(
  current: AssignmentActivityRuleDraft,
  saved: AssignmentActivityRules,
): AssignmentActivityRuleDraft {
  return {
    completionFraction:
      saved.assignmentCompletionRule.kind === "scoreAtLeast"
        ? numberDraft(saved.assignmentCompletionRule.fraction)
        : current.completionFraction,
    additionalAssignmentAttempts:
      saved.assignmentAttemptContinuationRule.kind === "capped"
        ? numberDraft(saved.assignmentAttemptContinuationRule.maxAdditionalAssignmentAttempts)
        : current.additionalAssignmentAttempts,
  };
}

export function numberDraft(value: number | null): string {
  return value === null ? "" : String(value);
}
