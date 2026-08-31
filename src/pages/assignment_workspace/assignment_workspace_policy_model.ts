// assignment_workspace_policy_model.ts - policy-page request construction and local control values.

import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { AssignmentPoliciesValidationIssue } from "../../../generated/api/AssignmentPoliciesValidationIssue";
import type { AssignmentPoliciesInput } from "../../api/contracts";

export type PolicyFocusTarget =
  | "lifecycle"
  | "instructions"
  | "availableAt"
  | "dueAt"
  | "closesAt"
  | "timeLimitSeconds"
  | "attemptLimit"
  | "completionFraction"
  | "additionalRuns"
  | "variation"
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
  disclosurePolicy: AssignmentPoliciesInput["disclosurePolicy"],
  policies: AssignmentPoliciesInput["policies"],
  teachingSettings: InstructorAssignmentTeachingSettingsLocal,
): AssignmentPoliciesInput {
  return { disclosurePolicy, policies, teachingSettings };
}

/** Converts a native local-date-time control value to the explicit wire form. */
export function canonicalCourseLocalTime(value: string): string | null {
  if (value === "") return null;
  const normalized = value.length === 16 ? `${value}:00.000` : value;
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(normalized) ? normalized : null;
}

function teachingTarget(
  field: AssignmentPoliciesValidationIssue & { kind: "teachingSettings" },
): PolicyFocusTarget {
  const target = field.correction.field;
  if (target === "instructions") return "instructions";
  if (target === "availableAt") return "availableAt";
  if (target === "dueAt") return "dueAt";
  if (target === "closesAt") return "closesAt";
  if (target === "timeLimitSeconds") return "timeLimitSeconds";
  if (target === "attemptLimit") return "attemptLimit";
  if (target === "lifecycle") return "lifecycle";
  return "schedule";
}

function teachingMessage(
  reason: AssignmentPoliciesValidationIssue & { kind: "teachingSettings" },
): string {
  switch (reason.correction.reason) {
    case "courseTimeZoneMismatch":
      return "Use the course time zone for this assignment schedule.";
    case "outsideCourseTerm":
      return "Choose assignment times within this course term.";
    case "nonexistentLocalTime":
    case "ambiguousLocalTime":
    case "timestampOutOfRange":
      return "Choose a valid course-local date and time.";
    case "scheduleOutOfOrder":
      return "Arrange the available, due, and close times in order.";
    case "timeLimitOutOfRange":
      return "Choose a valid whole Assignment Attempt time limit.";
    case "attemptLimitOutOfRange":
      return "Choose a valid attempt limit.";
    case "illegalLifecycleTransition":
      return "Choose a lifecycle that follows this assignment's current state.";
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
    case "perQuestionTiming":
      return "per-question timing";
    case "printExport":
      return "print export";
    case "offlinePreview":
      return "offline preview";
  }
}

function issueFeedback(issue: AssignmentPoliciesValidationIssue): AssignmentPolicySaveFeedback {
  switch (issue.kind) {
    case "teachingSettings":
      return {
        kind: "error",
        message: teachingMessage(issue),
        target: teachingTarget(issue),
        details: [],
        questionRepairRequired: false,
      };
    case "configuration":
      return {
        kind: "error",
        message: "Selected Question Variants require fixed Assignment Questions.",
        target: "variation",
        details: [
          "Choose a different next-practice Assignment Attempt rule or revise the Assignment Questions.",
        ],
        questionRepairRequired: false,
      };
    case "capability": {
      const variationTarget = issue.capability === "algorithmicGeneration";
      const detail = `${issue.title} needs ${capabilityLabel(issue)}.`;
      return {
        kind: "error",
        message: variationTarget
          ? "The variation policy needs a compatible question."
          : "One or more assignment questions need attention.",
        target: variationTarget ? "variation" : "questions",
        details: [detail],
        questionRepairRequired: !variationTarget,
      };
    }
    case "publicationReadiness":
      return {
        kind: "error",
        message: "Add at least one question before publishing this assignment.",
        target: "lifecycle",
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

export type AssignmentActivityRuleDraftField = "completionFraction" | "additionalRuns";

/** Raw number controls stay local until their typed policy value is valid. */
export type AssignmentActivityRuleDraft = {
  readonly completionFraction: string;
  readonly additionalRuns: string;
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

export function activityRuleDraftFromRules(policies: AssignmentActivityRules): AssignmentActivityRuleDraft {
  return {
    completionFraction: numberDraft(
      policies.completion.kind === "scoreAtLeast" ? policies.completion.fraction : 0.8,
    ),
    additionalRuns: numberDraft(
      policies.continuedPractice.kind === "capped"
        ? policies.continuedPractice.maxAdditionalRuns
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
      saved.completion.kind === "scoreAtLeast"
        ? numberDraft(saved.completion.fraction)
        : current.completionFraction,
    additionalRuns:
      saved.continuedPractice.kind === "capped"
        ? numberDraft(saved.continuedPractice.maxAdditionalRuns)
        : current.additionalRuns,
  };
}

export function numberDraft(value: number | null): string {
  return value === null ? "" : String(value);
}
